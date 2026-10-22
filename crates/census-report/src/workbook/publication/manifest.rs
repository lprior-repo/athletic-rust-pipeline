use super::invariant;
use crate::export::{DatasetLineage, ExportDataset};
use crate::report::{io_error, ReportResult, Scope};
use crate::workbook::Options;
use census_domain::model::{GradYear, SchoolYear};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

mod artifact;
mod comparison;
pub(in crate::workbook) use comparison::comparison_input;
mod counts;
mod decode;
#[cfg(test)]
pub(in crate::workbook::publication) mod historical_fixture;
#[cfg(test)]
pub(super) use counts::retained as retained_counts;
mod directory;
mod selection;
#[cfg(test)]
pub(in crate::workbook) mod tests;
mod validate;
mod verify;
use artifact::{artifact_names, hash_artifact, Artifact};
pub(super) use directory::verify_directory;
pub(super) use verify::verify_prepared;

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

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Manifest {
    schema_revision: u32,
    pub(super) lineage: DatasetLineage,
    selection: Selection,
    artifacts: BTreeMap<String, Artifact>,
    #[serde(
        default,
        deserialize_with = "counts::decode",
        skip_serializing_if = "Option::is_none"
    )]
    normalized_csv_rows: Option<super::inventory::RowCounts>,
    #[serde(skip_serializing_if = "Option::is_none")]
    selected_profile: Option<super::selected::SelectedProfile>,
    pub(super) generation_digest: String,
}

impl Manifest {
    pub(super) fn is_selected_job(&self) -> bool {
        self.selected_profile.is_some() && self.lineage.export_job.is_some()
    }

    pub(super) fn recipe(&self) -> ReportResult<census_store::publish::Digest> {
        super::selected::validate(self.schema_revision, self.selected_profile.as_ref())?;
        match &self.selected_profile {
            Some(selected) if self.schema_revision == super::selected::HISTORICAL_REVISION => {
                super::binding::recipe_for_selected(&self.lineage, &self.selection()?, selected)
            }
            Some(selected) => super::binding::recipe_for_selected_revision(
                &self.lineage,
                &self.selection()?,
                selected,
                self.schema_revision,
            ),
            None => super::binding::recipe_for_artifact(
                &self.lineage,
                &self.selection()?,
                self.schema_revision,
            ),
        }
    }

    pub(super) fn selection(&self) -> ReportResult<Options> {
        self.selection.options()
    }

    pub(super) fn reusable_for(&self, fresh: &Self) -> bool {
        self.schema_revision == fresh.schema_revision
            && self.lineage.input_generation == fresh.lineage.input_generation
            && self.selection == fresh.selection
            && self.normalized_csv_rows == fresh.normalized_csv_rows
            && self.selected_profile == fresh.selected_profile
    }

    pub(super) fn capture(
        directory: &Path,
        dataset: &ExportDataset,
        options: &Options,
        normalized_csv_rows: super::inventory::RowCounts,
    ) -> ReportResult<Self> {
        let revision = super::selected::revision(dataset)?;
        let selected_profile = super::selected::binding(revision, dataset)?;
        super::inventory::validate(revision, Some(&normalized_csv_rows))?;
        let selection = Selection::of(options);
        let names = artifact_names(revision, options.grad_year)?;
        let artifacts = names
            .into_iter()
            .map(|name| hash_artifact(&directory.join(&name)).map(|artifact| (name, artifact)))
            .collect::<ReportResult<BTreeMap<_, _>>>()?;
        let mut manifest = Self {
            schema_revision: revision,
            lineage: dataset.lineage.clone(),
            selection,
            artifacts,
            normalized_csv_rows: Some(normalized_csv_rows),
            selected_profile,
            generation_digest: String::new(),
        };
        manifest.generation_digest = manifest.digest()?;
        Ok(manifest)
    }

    #[cfg(test)]
    pub(in crate::workbook::publication) fn recapture(
        directory: &Path,
        dataset: &ExportDataset,
        options: &Options,
    ) -> ReportResult<Self> {
        Self::capture(directory, dataset, options, counts::retained(directory)?)
    }

    fn digest(&self) -> ReportResult<String> {
        super::selected::validate(self.schema_revision, self.selected_profile.as_ref())?;
        super::inventory::validate(self.schema_revision, self.normalized_csv_rows.as_ref())?;
        let result = match self.schema_revision {
            1..=4 => census_domain::model::serialized_digest(&(
                self.schema_revision,
                &self.lineage,
                &self.selection,
                &self.artifacts,
            )),
            5 | 6 => census_domain::model::serialized_digest(&(
                self.schema_revision,
                &self.lineage,
                &self.selection,
                &self.artifacts,
                &self.normalized_csv_rows,
            )),
            7..=9 => census_domain::model::serialized_digest(&(
                self.schema_revision,
                &self.lineage,
                &self.selection,
                &self.artifacts,
                &self.normalized_csv_rows,
                &self.selected_profile,
            )),
            _ => {
                return Err(invariant(
                    "unsupported publication artifact revision".into(),
                ))
            }
        };
        result.map_err(|error| invariant(format!("encoding generation manifest: {error}")))
    }

    fn ensure_dataset(&self, dataset: &ExportDataset) -> ReportResult<()> {
        if self.selected_profile != super::selected::binding(self.schema_revision, dataset)? {
            return Err(invariant(
                "manifest and frozen selected profile binding disagree".into(),
            ));
        }
        Ok(())
    }
}

pub fn current_workbook(root: &Path) -> ReportResult<PathBuf> {
    let (directory, _) = current_generation(root)?;
    Ok(directory.join("workbook.xlsx"))
}

pub(super) fn current_generation(root: &Path) -> ReportResult<(PathBuf, Manifest)> {
    let pointer = root.join("current");
    let relative = std::fs::read_link(&pointer).map_err(|source| io_error(&pointer, source))?;
    validate::validate_pointer(&relative)?;
    let generations = root.join("generations");
    let directory = root.join(relative);
    validate::validate_generation_paths(&generations, &directory)?;
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

pub struct OpenedPublication {
    pub dataset: ExportDataset,
    pub generation_digest: String,
    pub workbook: crate::workbook::verify::VerifiedWorkbook,
    pub has_selected_profile: bool,
}

pub fn verify_published(path: &Path) -> ReportResult<VerifiedPublication> {
    let (directory, manifest) = verify::preflight(path)?;
    let dataset = verify::decode(&directory, &manifest)?;
    let workbook = verify::verify_against(&directory, &manifest, &dataset)?;
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
    let (directory, manifest) = verify::preflight(path)?;
    super::seal::binding(&manifest, store_lineage, scope, cohort)?;
    let dataset = verify::decode(&directory, &manifest)?;
    let workbook = verify::verify_against(&directory, &manifest, &dataset)?;
    Ok(OpenedPublication {
        has_selected_profile: manifest.selected_profile.is_some(),
        dataset,
        generation_digest: manifest.generation_digest,
        workbook,
    })
}

pub fn open_publication_for_seal(
    path: &Path,
    store: &census_store::Store,
    scope: Scope,
    cohort: GradYear,
    jurisdictions: &[census_domain::UsJurisdiction],
) -> ReportResult<OpenedPublication> {
    super::incident::check_workbook(path)?;
    let (directory, manifest) = verify::preflight(path)?;
    let snapshot = store.snapshot();
    let current = crate::export::store_snapshot_lineage(store, &snapshot)?;
    super::seal::binding(&manifest, &current, scope, cohort)?;
    super::seal::require_selection(&manifest, jurisdictions)?;
    let dataset = verify::decode(&directory, &manifest)?;
    dataset.ensure_current_selected_authority()?;
    dataset.ensure_current_documentary_authority(&snapshot)?;
    let workbook = verify::verify_against(&directory, &manifest, &dataset)?;
    Ok(OpenedPublication {
        has_selected_profile: manifest.selected_profile.is_some(),
        dataset,
        generation_digest: manifest.generation_digest,
        workbook,
    })
}
