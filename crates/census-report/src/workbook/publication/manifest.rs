use super::{invariant, sidecars};
use crate::export::{DatasetLineage, ExportDataset};
use crate::report::{io_error, ReportResult, Scope};
use crate::workbook::Options;
use census_domain::model::SchoolYear;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

mod artifact;
use artifact::{hash_artifact, Artifact, MAX_BUNDLE_BYTES};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Selection {
    scope: String,
    grad_year: Option<i16>,
    limit: Option<usize>,
    school_year: Option<SchoolYear>,
}

impl Selection {
    fn of(options: &Options) -> Self {
        Self {
            scope: options.scope.as_str().to_string(),
            grad_year: options.grad_year,
            limit: options.limit,
            school_year: options.school_year,
        }
    }

    fn options(&self) -> ReportResult<Options> {
        let scope = match self.scope.as_str() {
            "core" => Scope::Core,
            "all_sources" => Scope::AllSources,
            _ => return Err(invariant("invalid publication scope".to_string())),
        };
        Ok(Options {
            scope,
            grad_year: self.grad_year,
            limit: self.limit,
            school_year: self.school_year,
            out: None,
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
    pub(super) fn capture(
        directory: &Path,
        dataset: &ExportDataset,
        options: &Options,
    ) -> ReportResult<Self> {
        let selection = Selection::of(options);
        let names = artifact_names(options.grad_year);
        let artifacts = names
            .into_iter()
            .map(|name| hash_artifact(&directory.join(&name)).map(|artifact| (name, artifact)))
            .collect::<ReportResult<BTreeMap<_, _>>>()?;
        let mut manifest = Self {
            schema_revision: 1,
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
    let generations = root.join("generations");
    let directory = root.join(relative);
    for path in [&generations, &directory] {
        let metadata = std::fs::symlink_metadata(path).map_err(|source| io_error(path, source))?;
        if !metadata.file_type().is_dir() {
            return Err(invariant(
                "publication generation must be a real directory".to_string(),
            ));
        }
    }
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

#[derive(Debug)]
pub struct VerifiedPublication {
    pub generation_digest: String,
    pub workbook: crate::workbook::verify::VerifiedWorkbook,
}

pub fn verify_published(path: &Path) -> ReportResult<VerifiedPublication> {
    let (manifest, workbook) = verified(path)?;
    Ok(VerifiedPublication {
        generation_digest: manifest.generation_digest,
        workbook,
    })
}

pub fn verify_for_seal(
    path: &Path,
    current: &ExportDataset,
    scope: Scope,
    grad_year: i16,
) -> ReportResult<VerifiedPublication> {
    let (manifest, workbook) = verified(path)?;
    let selection = manifest.selection.options()?;
    if selection.scope != scope
        || selection.grad_year != Some(grad_year)
        || selection.limit.is_some()
    {
        return Err(invariant(
            "seal requires a complete publication of the requested scope and cohort".to_string(),
        ));
    }
    if manifest.lineage.store_identity != current.lineage.store_identity
        || manifest.lineage.source_digest != current.lineage.source_digest
        || manifest.lineage.input_digest != current.lineage.input_digest
        || manifest.lineage.schema_revision != current.lineage.schema_revision
        || manifest.lineage.policy_revision != current.lineage.policy_revision
    {
        return Err(invariant(
            "publication is stale or belongs to different source evidence".to_string(),
        ));
    }
    Ok(VerifiedPublication {
        generation_digest: manifest.generation_digest,
        workbook,
    })
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

fn verified(path: &Path) -> ReportResult<(Manifest, crate::workbook::verify::VerifiedWorkbook)> {
    if path.file_name().and_then(|name| name.to_str()) != Some("workbook.xlsx") {
        return Err(invariant(
            "verification requires a manifested generation workbook".to_string(),
        ));
    }
    let directory = path
        .parent()
        .ok_or_else(|| invariant("workbook has no generation directory".to_string()))?;
    let manifest = verify_directory(directory)?;
    let dataset = ExportDataset::reopen_frozen(&directory.join("frozen-input.json"))?;
    if dataset.lineage != manifest.lineage {
        return Err(invariant(
            "manifest and frozen input lineage disagree".to_string(),
        ));
    }
    let options = manifest.selection.options()?;
    super::verify_sidecars::verify(directory, &dataset, &options)?;
    let workbook = crate::workbook::verify::verify_frozen(path, &dataset, &options)?;
    Ok((manifest, workbook))
}

pub(super) fn verify_directory(directory: &Path) -> ReportResult<Manifest> {
    let path = directory.join("manifest.json");
    let manifest: Manifest = sidecars::read_json(&path)?;
    if manifest.schema_revision != 1 || manifest.digest()? != manifest.generation_digest {
        return Err(invariant(
            "unsupported or corrupt generation manifest".to_string(),
        ));
    }
    manifest.selection.options()?;
    let expected = artifact_names(manifest.selection.grad_year);
    if manifest.artifacts.keys().cloned().collect::<Vec<_>>() != expected {
        return Err(invariant(
            "generation has an invalid artifact set".to_string(),
        ));
    }
    let mut total = 0_u64;
    manifest.artifacts.iter().try_for_each(|(name, expected)| {
        let actual = hash_artifact(&directory.join(name))?;
        if actual != *expected {
            return Err(invariant(format!("generation artifact mismatch: {name}")));
        }
        total = total
            .checked_add(actual.bytes)
            .ok_or_else(|| invariant("bundle byte counter exhausted".to_string()))?;
        if total > MAX_BUNDLE_BYTES {
            return Err(invariant(
                "generation exceeds 16 GiB byte budget".to_string(),
            ));
        }
        Ok(())
    })?;
    let mut entries = std::fs::read_dir(directory).map_err(|source| io_error(directory, source))?;
    entries.try_for_each(|entry| {
        let entry = entry.map_err(|source| io_error(directory, source))?;
        let name = entry.file_name();
        if name != "manifest.json"
            && !manifest
                .artifacts
                .contains_key(name.to_string_lossy().as_ref())
        {
            return Err(invariant(format!(
                "unexpected generation artifact: {}",
                name.to_string_lossy()
            )));
        }
        Ok(())
    })?;
    Ok(manifest)
}

fn artifact_names(grad_year: Option<i16>) -> Vec<String> {
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
        "workbook.xlsx".to_string(),
    ];
    names.sort_unstable();
    names
}
