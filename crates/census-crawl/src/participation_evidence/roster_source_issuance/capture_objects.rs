use super::manifest::CheckedRosterParserManifestV1;
use super::source::{budget, domain, original};
use crate::participation_evidence::provider_school::ProviderSchoolPacketV1;
use crate::participation_evidence::{bound, hash, invalid, json, MAX_PACKET};
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::roster_affiliation_authority::LocallyCheckedRosterIssuanceV1;
use census_store::{RosterSourceEvidenceV1, StoreResult};

mod format;
pub(super) mod version3;

pub fn prepare_retained_roster_capture_objects_v2<'a>(
    _evidence: &'a RosterSourceEvidenceV1,
    _parser: &CheckedRosterParserManifestV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<PreparedRosterCaptureObjectsV1<'a>> {
    grant.poison();
    Err(invalid("unsupported retained roster capture objects v2").into())
}

pub struct PreparedRosterCaptureObjectsV1<'a> {
    evidence: &'a RosterSourceEvidenceV1,
    metadata: Vec<u8>,
    interpretation: Vec<u8>,
    occurrence: String,
    acquired_at: String,
}

impl PreparedRosterCaptureObjectsV1<'_> {
    pub fn metadata_bytes(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<&[u8]> {
        self.evidence.require_original_grant(grant)?;
        Ok(&self.metadata)
    }

    pub fn interpretation_bytes(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<&[u8]> {
        self.evidence.require_original_grant(grant)?;
        Ok(&self.interpretation)
    }

    pub fn occurrence(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<&str> {
        self.evidence.require_original_grant(grant)?;
        Ok(&self.occurrence)
    }

    pub fn acquired_at(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<&str> {
        self.evidence.require_original_grant(grant)?;
        Ok(&self.acquired_at)
    }

    pub(super) fn require_proposal(
        &self,
        proposal: &LocallyCheckedRosterIssuanceV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> CrawlResult<()> {
        original(self.evidence, grant, |grant| {
            super::source::source_reference(&self.metadata, proposal.capture().manifest(), grant)?;
            super::source::source_reference(
                &self.interpretation,
                proposal.capture().interpretation(),
                grant,
            )?;
            if proposal.capture().occurrence().as_str() != self.occurrence
                || proposal.capture().acquired_at() != self.acquired_at
            {
                return Err(
                    invalid("retained roster capture occurrence or literal differs").into(),
                );
            }
            Ok(())
        })
    }
}

pub fn prepare_retained_roster_capture_objects<'a>(
    evidence: &'a RosterSourceEvidenceV1,
    parser: &CheckedRosterParserManifestV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<PreparedRosterCaptureObjectsV1<'a>> {
    evidence.require_original_grant(grant)?;
    let result = original(evidence, grant, |grant| {
        let parser = replay_parser(evidence, parser, grant)?;
        let held = evidence.retained_root(grant)?;
        let raw = held.payload().get().as_bytes();
        bound(raw.len(), MAX_PACKET, "retained capture object packet")?;
        grant.charge_bytes(raw.len()).map_err(domain)?;
        grant.charge_steps(1).map_err(domain)?;
        if held.root().schema != census_store::RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1
            || held.root().payload.digest.as_str() != hash(raw)
            || held.root().payload.bytes
                != u64::try_from(raw.len())
                    .map_err(|_| census_store::StoreError::CounterOverflow)?
        {
            return Err(invalid("capture object retained root mismatch").into());
        }
        let packet: ProviderSchoolPacketV1 = serde_json::from_slice(raw).map_err(json)?;
        budget::charge(&packet, grant)?;
        if hash(&packet.encode()?) != held.root().key {
            return Err(invalid("capture object packet key mismatch").into());
        }
        budget::charge(&packet, grant)?;
        budget::charge(packet.directory(), grant)?;
        budget::charge(packet.roster(), grant)?;
        original(evidence, grant, |_| packet.read())?;
        let body = packet.roster().body()?;
        derive(evidence, &packet, &parser, &body, grant)
    });
    if result.is_err() {
        grant.poison();
    }
    result
}

pub(super) fn derive<'a>(
    evidence: &'a RosterSourceEvidenceV1,
    packet: &ProviderSchoolPacketV1,
    parser: &CheckedRosterParserManifestV1,
    body: &str,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<PreparedRosterCaptureObjectsV1<'a>> {
    original(evidence, grant, |grant| {
        format::derive(evidence, packet, parser.retained(), body, grant)
    })
}

fn replay_parser(
    evidence: &RosterSourceEvidenceV1,
    parser: &CheckedRosterParserManifestV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<CheckedRosterParserManifestV1> {
    original(evidence, grant, |grant| {
        let retained = parser.retained();
        let bytes = retained.canonical_bytes();
        let cost = bytes
            .len()
            .checked_mul(2)
            .and_then(|size| size.checked_add(8192))
            .ok_or(census_store::StoreError::CounterOverflow)?;
        grant.charge_steps(1).map_err(domain)?;
        grant.charge_bytes(cost).map_err(domain)?;
        let claim = retained.parser_claim()?;
        evidence.require_original_grant(grant)?;
        Ok(super::manifest::assess_retained_roster_parser_manifest(
            bytes, &claim, grant,
        )?)
    })
}

pub(super) fn derive_retained<'a>(
    evidence: &'a RosterSourceEvidenceV1,
    packet: &ProviderSchoolPacketV1,
    parser: &super::manifest::RetainedRosterParserManifestV1,
    body: &str,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<PreparedRosterCaptureObjectsV1<'a>> {
    original(evidence, grant, |grant| {
        format::derive(evidence, packet, parser, body, grant)
    })
}
