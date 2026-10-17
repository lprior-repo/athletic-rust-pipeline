use super::{
    audit::RetainedAudit, invalid, source_binding::BoundRosterSourceViewV1,
    CurrentAffiliationResolution as Resolution, PinnedCurrentAffiliations,
    UnavailableAffiliation as Gap,
};
use crate::export::ExportDataset;
use crate::report::{ReportError, ReportResult};
use census_domain::affiliation_candidates::*;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{AthleteId, QueryProfileV1, SourceSubject};
use census_store::{StoreError, StoreResult};
use std::collections::{BTreeMap, BTreeSet};

mod encoding;
mod identity;
mod material;
mod physical;
mod selection;

pub(super) struct SourceAuthorityFamilyInventoryV1 {
    bytes: Vec<u8>,
    digest: String,
}

impl SourceAuthorityFamilyInventoryV1 {
    pub(super) fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub(super) fn digest(&self) -> &str {
        &self.digest
    }
}

pub(super) fn assess(
    dataset: &ExportDataset,
    profile: &QueryProfileV1,
    bound: &BoundRosterSourceViewV1<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> ReportResult<(PinnedCurrentAffiliations, SourceAuthorityFamilyInventoryV1)> {
    let result = assess_bound(dataset, profile, bound, grant, false);
    if result.is_err() {
        grant.poison();
    }
    result
}

pub(super) fn assess_historical(
    dataset: &ExportDataset,
    profile: &QueryProfileV1,
    bound: &BoundRosterSourceViewV1<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> ReportResult<(PinnedCurrentAffiliations, SourceAuthorityFamilyInventoryV1)> {
    let result = assess_bound(dataset, profile, bound, grant, true);
    if result.is_err() {
        grant.poison();
    }
    result
}

fn assess_bound(
    dataset: &ExportDataset,
    profile: &QueryProfileV1,
    bound: &BoundRosterSourceViewV1<'_>,
    grant: &mut AttachmentReplayGrantV1,
    historical: bool,
) -> ReportResult<(PinnedCurrentAffiliations, SourceAuthorityFamilyInventoryV1)> {
    bound.require_binding(dataset, profile, grant)?;
    let people = identity::physical_people(dataset, bound, grant)?;
    let identities = identity::index(dataset, &people, grant)?;
    let materials = bound
        .claims(grant)?
        .iter()
        .map(|claim| {
            if historical {
                material::read_historical(&identities, claim, grant)
            } else {
                material::read(&identities, claim, grant)
            }
        })
        .collect::<StoreResult<Vec<_>>>()?;
    let mut audit = RetainedAudit::default();
    let physical = physical::read(dataset, bound, &materials, &mut audit, grant)?;
    let bytes = encoding::encode(
        &(
            "roster-source-family-inventory-v1",
            materials
                .iter()
                .map(material::Capture::inventory)
                .collect::<Vec<_>>(),
            &physical,
            &people,
            &identities,
        ),
        grant,
    )?;
    let digest = hash(&bytes);
    let family = SourceAuthorityFamilyInventoryV1 { bytes, digest };
    let known = bound.declared_view(&family)?;
    let rows = selection::assess(
        dataset,
        profile,
        &materials,
        &physical,
        known,
        bound.acquisition_cutoff(),
        &mut audit,
        grant,
    )?;
    let pinned = PinnedCurrentAffiliations {
        input_generation: dataset.lineage.input_generation.clone(),
        profile_id: profile.id().map_err(StoreError::from)?,
        as_of: profile.input().as_of.clone(),
        rows,
        audit,
    };
    Ok((pinned, family))
}

fn charge(grant: &mut AttachmentReplayGrantV1, bytes: usize, steps: usize) -> StoreResult<()> {
    grant
        .charge_bytes(bytes)
        .map_err(census_store::EvidenceSubjectAttachmentStoreError::Domain)?;
    grant
        .charge_steps(steps)
        .map_err(|error| census_store::EvidenceSubjectAttachmentStoreError::Domain(error).into())
}

fn report_store(error: ReportError) -> StoreError {
    match error {
        ReportError::Store(error) => error,
        other => invalid(&other.to_string()),
    }
}

fn checked<T>(result: Result<T, impl std::fmt::Display>) -> StoreResult<T> {
    result.map_err(|error| invalid(&error.to_string()))
}

fn object(value: &str) -> StoreResult<ObjectRef<'_>> {
    checked(ObjectRef::parse(value))
}

fn hash(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests;
