use super::{binding, manifest::RetainedRosterParserManifestV1, policies, source};
use crate::participation_evidence::provider_school::CheckedProviderSchoolPacketV1;
use crate::participation_evidence::{invalid, json};
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{CanonicalSchool, CanonicalTeam};
use census_store::{ReviewedRosterSourceClaimV1, StoreResult};

mod assessment;
mod getters;
mod interpretation;
pub(super) mod kernel_adapter;

pub(super) struct CheckedOriginalRosterPacket<'a> {
    evidence: &'a census_store::RosterSourceEvidenceV1,
    packet: crate::participation_evidence::provider_school::ProviderSchoolPacketV1,
}

impl<'a> CheckedOriginalRosterPacket<'a> {
    #[cfg(test)]
    pub(super) fn read_for_test(
        evidence: &'a census_store::RosterSourceEvidenceV1,
        proposal: &census_domain::roster_affiliation_authority::LocallyCheckedRosterIssuanceV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> CrawlResult<Self> {
        Self::read(evidence, proposal, grant)
    }

    fn read(
        evidence: &'a census_store::RosterSourceEvidenceV1,
        proposal: &census_domain::roster_affiliation_authority::LocallyCheckedRosterIssuanceV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> CrawlResult<Self> {
        let held = evidence.retained_root(grant)?;
        let packet = source::packet(held, proposal, grant)?;
        Ok(Self { evidence, packet })
    }

    pub(super) fn evidence(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<&census_store::RosterSourceEvidenceV1> {
        self.evidence.require_original_grant(grant)?;
        Ok(self.evidence)
    }

    pub(super) fn packet(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<&crate::participation_evidence::provider_school::ProviderSchoolPacketV1> {
        self.evidence.require_original_grant(grant)?;
        Ok(&self.packet)
    }
}

pub struct HistoricalReviewedRosterSourceV2<'a> {
    claim: &'a ReviewedRosterSourceClaimV1,
    checked: CheckedProviderSchoolPacketV1,
    school: CanonicalSchool,
    program: CanonicalTeam,
    body: String,
    objects: super::capture_objects::PreparedRosterCaptureObjectsV1<'a>,
    interpretation: &'a [u8],
}

pub fn assess_reviewed_roster_source_historical_v2<'a>(
    claim: &'a ReviewedRosterSourceClaimV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<HistoricalReviewedRosterSourceV2<'a>> {
    let result = assessment::assess(claim, grant);
    if result.is_err() {
        grant.poison();
    }
    result
}
