use super::invariant;
use crate::export::{DatasetLineage, ExportDataset};
use crate::report::{io_error, ReportResult, Scope};
use crate::workbook::Options;
use census_domain::model::{GradYear, SchoolYear};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

mod artifact;
mod directory;
#[cfg(test)]
mod tests;
use artifact::{hash_artifact, Artifact};
pub(super) use directory::verify_directory;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Selection {
    scope: String,
    grad_year: Option<i16>,
    limit: Option<usize>,
    school_year: Option<SchoolYear>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    jurisdictions: Vec<String>,
}

impl Selection {
    fn of(options: &Options) -> Self {
        let mut jurisdictions: Vec<String> = options
            .jurisdictions
            .iter()
            .map(|jurisdiction| jurisdiction.code().to_string())
            .collect();
        jurisdictions.sort();
        jurisdictions.dedup();
        Self {
            scope: options.scope.as_str().to_string(),
            grad_year: options.grad_year,
            limit: options.limit,
            school_year: Some(options.school_year),
            jurisdictions,
        }
    }

    fn options(&self) -> ReportResult<Options> {
        let scope = match self.scope.as_str() {
            "core" => Scope::Core,
            "all_sources" => Scope::AllSources,
            _ => return Err(invariant("invalid publication scope".to_string())),
        };
        let school_year = self.school_year.ok_or_else(|| {
            invariant("the publication manifest carries no contact school year".to_string())
        })?;
        let jurisdictions = self
            .jurisdictions
            .iter()
            .map(|code| {
                census_domain::UsJurisdiction::from_code(code).ok_or_else(|| {
                    invariant(format!(
                        "the publication manifest carries an unknown jurisdiction {code:?}"
                    ))
                })
            })
            .collect::<ReportResult<Vec<_>>>()?;
        Ok(Options {
            scope,
            grad_year: self.grad_year,
            limit: self.limit,
            school_year,
            out: None,
            jurisdictions,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Manifest {
    schema_revision: u32,
    pub(super) lineage: DatasetLineage,
    selection: Selection,
    artifacts: BTreeMap<String, Artifact>,
    pub(super) generation_digest: String,
}

impl Manifest {
    pub(super) fn selection(&self) -> ReportResult<Options> {
        self.selection.options()
    }

    pub(super) fn capture(
        directory: &Path,
        dataset: &ExportDataset,
        options: &Options,
    ) -> ReportResult<Self> {
        let selection = Selection::of(options);
        let names = artifact_names(2, options.grad_year)?;
        let artifacts = names
            .into_iter()
            .map(|name| hash_artifact(&directory.join(&name)).map(|artifact| (name, artifact)))
            .collect::<ReportResult<BTreeMap<_, _>>>()?;
        let mut manifest = Self {
            schema_revision: 2,
            lineage: dataset.lineage.clone(),
            selection,
            artifacts,
            generation_digest: String::new(),
        };
        manifest.generation_digest = manifest.digest()?;
        Ok(manifest)
    }

    fn digest(&self) -> ReportResult<String> {
        census_domain::model::serialized_digest(&(
            self.schema_revision,
            &self.lineage,
            &self.selection,
            &self.artifacts,
        ))
        .map_err(|error| invariant(format!("encoding generation manifest: {error}")))
    }
}

pub fn current_workbook(root: &Path) -> ReportResult<PathBuf> {
    let (directory, _) = current_generation(root)?;
    Ok(directory.join("workbook.xlsx"))
}

pub(super) fn current_generation(root: &Path) -> ReportResult<(PathBuf, Manifest)> {
    let pointer = root.join("current");
    let relative = std::fs::read_link(&pointer).map_err(|source| io_error(&pointer, source))?;
    validate_pointer(&relative)?;
    let generations = root.join("generations");
    let directory = root.join(relative);
    validate_generation_paths(&generations, &directory)?;
    let directory =
        std::fs::canonicalize(&directory).map_err(|source| io_error(&directory, source))?;
    let manifest = verify_directory(&directory)?;
    if directory.file_name().and_then(|name| name.to_str())
        != Some(manifest.generation_digest.as_str())
    {
        return Err(invariant(
            "publication directory does not match its manifest".to_string(),
        ));
    }
    Ok((directory, manifest))
}

fn validate_pointer(relative: &Path) -> ReportResult<()> {
    let mut components = relative.components();
    let valid = components.next()
        == Some(std::path::Component::Normal(std::ffi::OsStr::new(
            "generations",
        )))
        && components
            .next()
            .is_some_and(|part| matches!(part, std::path::Component::Normal(_)))
        && components.next().is_none();
    if !valid {
        return Err(invariant(
            "publication pointer escapes its generation directory".to_string(),
        ));
    }
    Ok(())
}

fn validate_generation_paths(generations: &Path, directory: &Path) -> ReportResult<()> {
    for path in [generations, directory] {
        let metadata = std::fs::symlink_metadata(path).map_err(|source| io_error(path, source))?;
        if !metadata.file_type().is_dir() {
            return Err(invariant(
                "publication generation must be a real directory".to_string(),
            ));
        }
    }
    Ok(())
}

#[derive(Debug)]
pub struct VerifiedPublication {
    pub generation_digest: String,
    pub workbook: crate::workbook::verify::VerifiedWorkbook,
}

pub struct OpenedPublication {
    pub dataset: ExportDataset,
    pub generation_digest: String,
    pub workbook: crate::workbook::verify::VerifiedWorkbook,
}

pub fn verify_published(path: &Path) -> ReportResult<VerifiedPublication> {
    let (directory, manifest) = preflight(path)?;
    let dataset = decode(&directory, &manifest)?;
    let workbook = verify_against(&directory, &manifest, &dataset)?;
    Ok(VerifiedPublication {
        generation_digest: manifest.generation_digest,
        workbook,
    })
}

pub fn open_publication(
    path: &Path,
    store_lineage: &DatasetLineage,
    scope: Scope,
    cohort: GradYear,
) -> ReportResult<OpenedPublication> {
    super::incident::check_workbook(path)?;
    let (directory, manifest) = preflight(path)?;
    super::seal::binding(&manifest, store_lineage, scope, cohort)?;
    let dataset = decode(&directory, &manifest)?;
    let workbook = verify_against(&directory, &manifest, &dataset)?;
    Ok(OpenedPublication {
        dataset,
        generation_digest: manifest.generation_digest,
        workbook,
    })
}

pub fn open_publication_for_seal(
    path: &Path,
    store_lineage: &DatasetLineage,
    scope: Scope,
    cohort: GradYear,
    jurisdictions: &[census_domain::UsJurisdiction],
) -> ReportResult<OpenedPublication> {
    super::incident::check_workbook(path)?;
    let (directory, manifest) = preflight(path)?;
    super::seal::binding(&manifest, store_lineage, scope, cohort)?;
    super::seal::require_selection(&manifest, jurisdictions)?;
    let dataset = decode(&directory, &manifest)?;
    let workbook = verify_against(&directory, &manifest, &dataset)?;
    Ok(OpenedPublication {
        dataset,
        generation_digest: manifest.generation_digest,
        workbook,
    })
}

fn decode(directory: &Path, manifest: &Manifest) -> ReportResult<ExportDataset> {
    let dataset = ExportDataset::reopen_frozen(&directory.join("frozen-input.json"))?;
    if dataset.lineage != manifest.lineage {
        return Err(invariant(
            "manifest and frozen input lineage disagree".to_string(),
        ));
    }
    Ok(dataset)
}

pub(super) fn verify_prepared(
    directory: &Path,
    dataset: &ExportDataset,
    options: &Options,
) -> ReportResult<()> {
    let manifest = verify_directory(directory)?;
    if manifest.lineage != dataset.lineage || manifest.selection != Selection::of(options) {
        return Err(invariant(
            "prepared publication differs from its captured input or selection".to_string(),
        ));
    }
    let frozen = manifest
        .artifacts
        .get("frozen-input.json")
        .ok_or_else(|| invariant("publication lacks its frozen input".to_string()))?;
    if frozen.sha256 != dataset.archive_digest()? {
        return Err(invariant(
            "retained frozen bytes differ from the captured export input".to_string(),
        ));
    }
    super::verify_sidecars::verify(directory, dataset, options)?;
    crate::workbook::verify::verify_frozen(&directory.join("workbook.xlsx"), dataset, options)?;
    Ok(())
}

fn preflight(path: &Path) -> ReportResult<(PathBuf, Manifest)> {
    if path.file_name().and_then(|name| name.to_str()) != Some("workbook.xlsx") {
        return Err(invariant(
            "verification requires a manifested generation workbook".to_string(),
        ));
    }
    let directory = path
        .parent()
        .ok_or_else(|| invariant("workbook has no generation directory".to_string()))?;
    let manifest = verify_directory(directory)?;
    Ok((directory.to_path_buf(), manifest))
}

fn verify_against(
    directory: &Path,
    manifest: &Manifest,
    dataset: &ExportDataset,
) -> ReportResult<crate::workbook::verify::VerifiedWorkbook> {
    let options = manifest.selection.options()?;
    super::verify_sidecars::verify(directory, dataset, &options)?;
    crate::workbook::verify::verify_frozen(&directory.join("workbook.xlsx"), dataset, &options)
}

fn artifact_names(revision: u32, grad_year: Option<i16>) -> ReportResult<Vec<String>> {
    let cohort = match grad_year.map(|year| format!("co{year}")) {
        Some(value) => value,
        None => "all".to_string(),
    };
    let mut names = vec![
        "audit.json".to_string(),
        format!("best-results-{cohort}.csv"),
        format!("best-results-{cohort}.jsonl"),
        "census-all-sources.json".to_string(),
        "census-core.json".to_string(),
        "frozen-input.json".to_string(),
        "recruiting.csv".to_string(),
        "school-contacts.csv".to_string(),
        "contact-research.csv".to_string(),
        "workbook.xlsx".to_string(),
    ];
    match revision {
        1 => {}
        2 => names.extend(
            [
                "event-catalog.json",
                "relay-dispositions.json",
                "relays.jsonl",
            ]
            .map(str::to_string),
        ),
        _ => {
            return Err(invariant(
                "unsupported publication artifact revision".into(),
            ))
        }
    }
    names.sort_unstable();
    Ok(names)
}
