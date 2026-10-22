use super::super::{
    codec, inspection::RosterIssuanceInspectionV1, RosterSourceIssuanceStoreError as Error,
};
use super::{ObjectRef, RosterSourceReviewObjectsV1};
use crate::StoreResult;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::roster_affiliation_authority::{
    LocallyCheckedRosterIssuanceV1, LocallyCheckedRosterProposalV1, RosterAuthorityObjectRefV1,
};
use std::collections::BTreeMap;

pub(super) type Objects = BTreeMap<String, Vec<u8>>;
mod builder;
pub(super) use builder::Builder;

pub(super) fn insert(
    objects: &mut Builder,
    bytes: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<ObjectRef> {
    insert_bounded(objects, bytes, 4 * 1024 * 1024, grant)
}

fn insert_bounded(
    objects: &mut Builder,
    bytes: &[u8],
    maximum: usize,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<ObjectRef> {
    let (reference, admitted_total) = admit(objects, bytes, maximum, grant)?;
    if let Some(admitted_total) = admitted_total {
        let mut owned = Vec::new();
        owned
            .try_reserve_exact(bytes.len())
            .map_err(|_| Error::BudgetExceeded)?;
        owned.extend_from_slice(bytes);
        objects.retain(reference.digest.clone(), owned, admitted_total)?;
    }
    Ok(reference)
}

pub(super) fn insert_owned(
    objects: &mut Builder,
    bytes: Vec<u8>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<ObjectRef> {
    let (reference, admitted_total) = admit(objects, &bytes, 4 * 1024 * 1024, grant)?;
    if let Some(admitted_total) = admitted_total {
        objects.retain(reference.digest.clone(), bytes, admitted_total)?;
    }
    Ok(reference)
}

fn admit(
    objects: &Builder,
    bytes: &[u8],
    maximum: usize,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<(ObjectRef, Option<usize>)> {
    if bytes.is_empty()
        || bytes.len() > maximum
        || objects.len() >= census_domain::roster_affiliation_authority::MAX_ROSTER_CLOSURE_OBJECTS
    {
        return Err(Error::BudgetExceeded.into());
    }
    grant
        .charge_bytes(bytes.len())
        .map_err(super::super::source_error)?;
    grant.charge_steps(1).map_err(super::super::source_error)?;
    let digest = codec::digest(bytes);
    let required = if let Some(existing) = objects.get(&digest) {
        if existing != bytes {
            return Err(Error::Corrupt("object digest collision").into());
        }
        None
    } else {
        Some(objects.admit_bytes(bytes.len())?)
    };
    Ok((
        ObjectRef {
            digest,
            bytes: bytes.len(),
        },
        required,
    ))
}

pub(super) fn collect(
    inspection: &mut RosterIssuanceInspectionV1<'_>,
    supplied: RosterSourceReviewObjectsV1,
    proposal: &LocallyCheckedRosterProposalV1,
) -> StoreResult<Builder> {
    let mut objects = Builder::new();
    let (evidence, grant) = inspection.source_parts()?;
    retain_contracts(&mut objects, grant, supplied, proposal)?;
    retain_root_payload(&mut objects, evidence, grant, proposal)?;
    retain_descriptors(&mut objects, evidence, grant, proposal)?;
    retain_observed_admission(&mut objects, evidence, grant)?;
    evidence.require_original_grant(grant)?;
    Ok(objects)
}

fn retain_observed_admission(
    objects: &mut Builder,
    evidence: &super::super::inspection::RosterSourceEvidenceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    evidence.require_original_grant(grant)?;
    if let Some(proof) = evidence.observed.as_ref() {
        insert(objects, proof.mint_bytes(), grant)?;
        insert(objects, proof.receipt_bytes(), grant)?;
    }
    Ok(())
}

fn retain_contracts(
    objects: &mut Builder,
    grant: &mut AttachmentReplayGrantV1,
    supplied: RosterSourceReviewObjectsV1,
    proposal: &LocallyCheckedRosterProposalV1,
) -> StoreResult<()> {
    for (bytes, reference) in [
        (supplied.scope_policy, proposal.scope().policy()),
        (
            supplied.year_convention,
            proposal.binding().year_convention(),
        ),
        (
            supplied.projection_merge_contract,
            proposal.binding().projection_merge_contract(),
        ),
    ] {
        if bytes.len() > 4096 {
            return Err(Error::BudgetExceeded.into());
        }
        let actual = insert(objects, &bytes, grant)?;
        if reference.revision().get() != 1 || actual.digest != reference.digest() {
            return Err(Error::Corrupt("retained review contract mismatch").into());
        }
    }
    for bytes in supplied.objects {
        let manifest = proposal.binding().physical_projection_inventory();
        let large_manifest = bytes.len() > 4 * 1024 * 1024
            && u64::try_from(bytes.len()).map_err(|_| Error::BudgetExceeded)?
                == manifest.bytes().get();
        let maximum = if large_manifest {
            census_domain::roster_affiliation_authority::MAX_ROSTER_PHYSICAL_PROJECTION_MANIFEST_BYTES
        } else {
            4 * 1024 * 1024
        };
        let actual = insert_bounded(objects, &bytes, maximum, grant)?;
        if large_manifest && actual.digest != manifest.digest().as_str() {
            return Err(Error::Corrupt("supplied physical manifest differs").into());
        }
    }
    Ok(())
}

fn retain_root_payload(
    objects: &mut Builder,
    evidence: &super::super::inspection::RosterSourceEvidenceV1,
    grant: &mut AttachmentReplayGrantV1,
    proposal: &LocallyCheckedRosterProposalV1,
) -> StoreResult<()> {
    let root = evidence.retained_root(grant)?;
    if proposal.binding().provider_packet().digest().as_str() != root.root().key {
        return Err(Error::Corrupt("review provider root differs").into());
    }
    if root.payload().get().len() > 4 * 1024 * 1024 {
        return Err(Error::BudgetExceeded.into());
    }
    insert(objects, root.payload().get().as_bytes(), grant)?;
    for row in evidence.physical_projection(grant)? {
        insert(objects, row.bytes(), grant)?;
    }
    Ok(())
}

fn retain_descriptors(
    objects: &mut Builder,
    evidence: &super::super::inspection::RosterSourceEvidenceV1,
    grant: &mut AttachmentReplayGrantV1,
    proposal: &LocallyCheckedRosterProposalV1,
) -> StoreResult<()> {
    for (bytes, expected, maximum) in [
        (
            evidence.physical_projection_manifest(grant)?,
            proposal.binding().physical_projection_inventory(),
            census_domain::roster_affiliation_authority::MAX_ROSTER_PHYSICAL_PROJECTION_MANIFEST_BYTES,
        ),
        (
            evidence.retained_source_admission_manifest(grant)?,
            proposal.capture().retained_source_admission(),
            4 * 1024 * 1024,
        ),
        (
            evidence.verification_grant_manifest(grant)?,
            proposal.capture().verification_grant(),
            4 * 1024 * 1024,
        ),
    ] {
        let actual = insert_bounded(objects, &bytes, maximum, grant)?;
        if actual.digest != expected.digest().as_str()
            || u64::try_from(actual.bytes).map_err(|_| Error::BudgetExceeded)?
                != expected.bytes().get()
        {
            return Err(Error::Corrupt("review inspection descriptor differs").into());
        }
    }
    Ok(())
}

pub(super) fn require_proposal(
    objects: &Objects,
    proposal: &LocallyCheckedRosterIssuanceV1,
) -> StoreResult<()> {
    let parser = proposal.parser();
    let capture = proposal.capture();
    let binding = proposal.binding();
    for reference in [
        parser.artifact(),
        parser.schema(),
        parser.configuration(),
        parser.dependencies(),
        capture.body(),
        capture.manifest(),
        capture.interpretation(),
        capture.retained_source_admission(),
        capture.verification_grant(),
        binding.school_projection(),
        binding.program_projection(),
        binding.provider_packet(),
        binding.physical_projection_inventory(),
        proposal.semantic_review_evidence(),
    ] {
        require(objects, reference)?;
    }
    Ok(())
}

fn require(objects: &Objects, reference: &RosterAuthorityObjectRefV1) -> StoreResult<()> {
    let bytes = objects
        .get(reference.digest().as_str())
        .ok_or(Error::Corrupt("missing reviewed object"))?;
    if u64::try_from(bytes.len()).map_err(|_| Error::BudgetExceeded)? != reference.bytes().get() {
        return Err(Error::Corrupt("reviewed object length").into());
    }
    Ok(())
}

pub(super) fn inventory(objects: &Objects) -> Vec<ObjectRef> {
    objects
        .iter()
        .map(|(digest, bytes)| ObjectRef {
            digest: digest.clone(),
            bytes: bytes.len(),
        })
        .collect()
}

#[cfg(test)]
mod tests;
