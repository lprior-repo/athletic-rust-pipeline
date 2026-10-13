use super::{PublicationError, ReportResult};
use crate::export::ExportDataset;
use crate::workbook::Options;
use census_store::publish::{Digest, InputFence};
use census_store::Store;
use serde::Serialize;
use std::path::Path;

pub(super) fn digest(value: &impl Serialize) -> ReportResult<Digest> {
    let value = census_domain::model::serialized_digest(value)
        .map_err(|error| super::invariant(format!("encoding publication binding: {error}")))?;
    Ok(Digest::parse(&value)?)
}
pub(super) fn namespace(root: &Path) -> ReportResult<Digest> {
    census_store::fs::create_dir_all_synced(root)
        .map_err(|source| super::io_error(root, source))?;
    let canonical = std::fs::canonicalize(root).map_err(|source| super::io_error(root, source))?;
    digest(&canonical)
}
pub(super) fn run_manifest(store: &Store) -> ReportResult<Option<Digest>> {
    store.run_manifest()?.as_ref().map(digest).transpose()
}
pub(super) fn input_token(lineage: &crate::export::DatasetLineage) -> ReportResult<Digest> {
    digest(&(
        &lineage.store_identity,
        &lineage.input_generation,
        lineage.run,
        lineage.cohort,
    ))
}
pub(super) fn input(
    dataset: &ExportDataset,
    store: &Store,
    options: &Options,
) -> ReportResult<InputFence> {
    dataset.ensure_store(store).map_err(input_refused)?;
    let snapshot = store.snapshot();
    dataset.ensure_snapshot(&snapshot).map_err(input_refused)?;
    let manifest = snapshot.run_manifest()?;
    if manifest.as_ref().map(|value| value.run) != dataset.lineage.run
        || manifest.as_ref().map(|value| value.cohort) != dataset.lineage.cohort
    {
        return Err(PublicationError::InputFenceRefused {
            detail: "bound run differs from frozen input lineage".into(),
        }
        .into());
    }
    Ok(InputFence {
        lineage: Digest::parse(&dataset.lineage.store_identity)?,
        token: input_token(&dataset.lineage)?,
        recipe: recipe(&dataset.lineage, options)?,
        run_manifest: manifest.as_ref().map(digest).transpose()?,
        evidence_generation: snapshot.evidence_generation(),
        derived_generation: snapshot.derived_generation(),
    })
}
pub(super) fn recipe(
    lineage: &crate::export::DatasetLineage,
    options: &Options,
) -> ReportResult<Digest> {
    let mut jurisdictions: Vec<&str> = options
        .jurisdictions
        .iter()
        .map(|jurisdiction| jurisdiction.code())
        .collect();
    jurisdictions.sort_unstable();
    jurisdictions.dedup();
    digest(&(
        options.scope.as_str(),
        options.grad_year,
        options.limit,
        options.school_year,
        jurisdictions,
        lineage.schema_revision,
        lineage.policy_revision,
    ))
}
pub(super) fn refused(error: census_store::StoreError) -> crate::report::ReportError {
    match error {
        census_store::StoreError::EvidenceMoved { .. }
        | census_store::StoreError::LeaseExpired { .. }
        | census_store::StoreError::LeaseMismatch { .. }
        | census_store::StoreError::PublicationRefused { .. } => {
            PublicationError::InputFenceRefused {
                detail: error.to_string(),
            }
            .into()
        }
        other => other.into(),
    }
}
pub(super) fn input_refused(error: crate::report::ReportError) -> crate::report::ReportError {
    match error {
        crate::report::ReportError::Invariant { detail } => {
            PublicationError::InputFenceRefused { detail }.into()
        }
        other => other,
    }
}
