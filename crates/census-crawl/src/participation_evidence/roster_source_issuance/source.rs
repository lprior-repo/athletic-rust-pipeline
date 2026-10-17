use super::{manifest::assess_retained_roster_parser_manifest, RosterSourceEvidenceAssessmentV1};
use crate::milesplit::{CurrentRosterAssertion, RosterVerdict};
use crate::participation_evidence::provider_school::{
    CheckedProviderSchoolPacketV1, ProviderSchoolPacketV1,
};
use crate::participation_evidence::{bound, hash, invalid, json, MAX_PACKET};
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::{
    AttachmentReplayGrantV1, EvidenceSubjectAttachmentError,
};
use census_domain::roster_affiliation_authority::{LocallyCheckedRosterIssuanceV1, RosterAuthorityObjectRefV1};
use census_store::{
    EvidenceSubjectAttachmentStoreError, RetainedCaptureRootReadV1,
    RetainedCaptureRootSchemaV1, RosterSourceEvidenceV1, StoreError,
};
use chrono::{DateTime, Utc};

#[path = "source_budget.rs"]
pub(super) mod budget;

pub fn assess_retained_roster_source<'a>(
    evidence: &'a RosterSourceEvidenceV1,
    retained_parser_manifest: &[u8],
    retained_contracts: &super::policies::RosterInterpretationContractsV1<'_>,
    proposal: &LocallyCheckedRosterIssuanceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<RosterSourceEvidenceAssessmentV1<'a>> {
    evidence.require_original_grant(grant)?;
    let result = assess(evidence, retained_parser_manifest, retained_contracts, proposal, grant);
    if result.is_err() {
        grant.poison();
    }
    result
}

fn assess<'a>(
    evidence: &'a RosterSourceEvidenceV1,
    retained_parser_manifest: &[u8],
    retained_contracts: &super::policies::RosterInterpretationContractsV1<'_>,
    proposal: &LocallyCheckedRosterIssuanceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<RosterSourceEvidenceAssessmentV1<'a>> {
    original(evidence, grant, |_| {
        if evidence.requested_school() != proposal.binding().school()
            || evidence.requested_program() != proposal.binding().program()
            || evidence.store_identity() != proposal.binding().store()
            || evidence.snapshot_sequence() != proposal.binding().snapshot_revision().get()
        {
            return Err(invalid("roster source snapshot lineage mismatch").into());
        }
        Ok(())
    })?;
    original(evidence, grant, |grant| Ok(super::policies::assess(evidence, retained_contracts, proposal, grant)?))?;
    let parser_manifest = original(evidence, grant, |grant| {
        Ok(assess_retained_roster_parser_manifest(retained_parser_manifest, proposal.parser(), grant)?)
    })?;
    original(evidence, grant, |grant| admission(evidence, proposal, grant))?;
    let held = evidence.retained_root(grant)?;
    let packet = original(evidence, grant, |grant| packet(held, proposal, grant))?;
    original(evidence, grant, |grant| capture(&packet, proposal, grant))?;
    let checked = original(evidence, grant, |grant| {
        budget::charge(&packet, grant)?;
        budget::charge(packet.directory(), grant)?;
        budget::charge(packet.roster(), grant)?;
        packet.read()
    })?;
    let objects = super::capture_objects::derive(evidence, &packet, &parser_manifest, grant)?;
    objects.require_proposal(proposal, grant)?;
    let scoped_rows = original(evidence, grant, |grant| scope(&checked, proposal, grant))?;
    evidence.require_original_grant(grant)?;
    Ok(RosterSourceEvidenceAssessmentV1 {
        evidence,
        parser_manifest,
        proposal: proposal.clone(),
        root: held.root().clone(),
        checked,
        scoped_rows,
        capture_objects: objects,
    })
}

pub(super) fn original<T>(
    evidence: &RosterSourceEvidenceV1,
    grant: &mut AttachmentReplayGrantV1,
    operation: impl FnOnce(&mut AttachmentReplayGrantV1) -> CrawlResult<T>,
) -> CrawlResult<T> {
    evidence.require_original_grant(grant)?;
    let result = operation(grant);
    evidence.require_original_grant(grant)?;
    result
}

fn admission(
    evidence: &RosterSourceEvidenceV1,
    proposal: &LocallyCheckedRosterIssuanceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<()> {
    let admission = evidence.retained_source_admission_manifest(grant)?;
    source_reference(&admission, proposal.capture().retained_source_admission(), grant)?;
    evidence.require_original_grant(grant)?;
    let verification = evidence.verification_grant_manifest(grant)?;
    source_reference(&verification, proposal.capture().verification_grant(), grant)?;
    evidence.require_original_grant(grant)?;
    Ok(())
}

pub(super) fn source_reference(
    bytes: &[u8],
    reference: &RosterAuthorityObjectRefV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<()> {
    grant.charge_steps(1).map_err(domain)?;
    grant.charge_bytes(bytes.len()).map_err(domain)?;
    if reference.bytes().get() != size(bytes.len())?
        || reference.digest().as_str() != hash(bytes)
    {
        return Err(invalid("roster source context object reference mismatch").into());
    }
    Ok(())
}

fn packet(
    held: &RetainedCaptureRootReadV1,
    proposal: &LocallyCheckedRosterIssuanceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<ProviderSchoolPacketV1> {
    let root = held.root();
    let bytes = held.payload().get().as_bytes();
    bound(bytes.len(), MAX_PACKET, "roster source packet")?;
    grant.charge_bytes(bytes.len()).map_err(domain)?;
    grant.charge_steps(1).map_err(domain)?;
    if root.schema != RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1
        || root.payload.bytes != size(bytes.len())?
        || root.payload.digest.as_str() != hash(bytes)
    {
        return Err(invalid("roster source retained root mismatch").into());
    }
    let packet: ProviderSchoolPacketV1 = serde_json::from_slice(bytes).map_err(json)?;
    budget::charge(&packet, grant)?;
    let canonical = packet.encode()?;
    let expected = proposal.binding().provider_packet();
    if root.key != hash(&canonical)
        || expected.digest().as_str() != root.key
        || expected.bytes().get() != size(canonical.len())?
    {
        return Err(invalid("roster source provider packet mismatch").into());
    }
    Ok(packet)
}

fn capture(
    packet: &ProviderSchoolPacketV1,
    proposal: &LocallyCheckedRosterIssuanceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<()> {
    let expected = proposal.capture();
    let retained = packet.roster();
    let expected_bytes = usize::try_from(expected.body().bytes().get())
        .map_err(|_| StoreError::CounterOverflow)?;
    budget::charge(retained, grant)?;
    let body = retained.body()?;
    let instant = DateTime::parse_from_rfc3339(expected.acquired_at())
        .map_err(|_| invalid("roster source acquisition instant"))?
        .with_timezone(&Utc);
    if expected.body().digest().as_str() != retained.body_sha256()
        || expected_bytes != body.len()
        || expected.requested_url() != retained.requested_url()
        || expected.observed_final_url() != Some(retained.requested_url())
        || instant != retained.acquired_at()
        || expected.observed_day().as_str() != retained.acquired_at().date_naive().to_string()
    {
        return Err(invalid("roster source capture mismatch").into());
    }
    Ok(())
}

fn scope(
    checked: &CheckedProviderSchoolPacketV1,
    proposal: &LocallyCheckedRosterIssuanceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<usize> {
    let RosterVerdict::Complete { roster } = checked.roster_verdict() else {
        return Err(invalid("roster source incomplete roster").into());
    };
    grant.charge_steps(roster.athletes.len()).map_err(domain)?;
    if roster.team.id != proposal.scope().owner().provider_id()
        || proposal.parser().interpretation() != CurrentRosterAssertion::INTERPRETATION_REVISION
    {
        return Err(invalid("roster source owner or interpretation mismatch").into());
    }
    let count = roster.athletes.iter().try_fold(0_usize, |count, member| {
        let assertion = member.current_roster()
            .ok_or_else(|| invalid("roster source current statement absent"))?;
        if assertion.owner() != roster.team.id
            || assertion.member() != member.athlete().athlete_id
            || assertion.row() != member.locator()
        {
            return Err(invalid("roster source scoped member mismatch"));
        }
        if member.athlete().gender == proposal.scope().category()
            && assertion.supports(proposal.scope().sport())
        {
            count.checked_add(1).ok_or(StoreError::CounterOverflow)
        } else {
            Ok(count)
        }
    })?;
    if count == 0 {
        return Err(invalid("roster source program scope absent").into());
    }
    Ok(count)
}

fn size(value: usize) -> Result<u64, StoreError> {
    u64::try_from(value).map_err(|_| StoreError::CounterOverflow)
}

pub(super) fn domain(error: EvidenceSubjectAttachmentError) -> StoreError {
    EvidenceSubjectAttachmentStoreError::Domain(error).into()
}

#[cfg(test)]
#[path = "source_tests.rs"]
mod tests;
