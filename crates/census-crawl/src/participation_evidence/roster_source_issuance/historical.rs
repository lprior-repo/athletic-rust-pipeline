use super::{
    binding, manifest::RetainedRosterParserManifestV1, policies, semantic_inventory, source,
};
use crate::participation_evidence::provider_school::CheckedProviderSchoolPacketV1;
use crate::participation_evidence::{invalid, json};
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{CanonicalSchool, CanonicalTeam};
use census_store::{ReviewedRosterSourceClaimV1, StoreResult};

mod assessment;
mod getters;
mod interpretation;

pub struct HistoricalReviewedRosterSourceV2<'a> {
    claim: &'a ReviewedRosterSourceClaimV1,
    checked: CheckedProviderSchoolPacketV1,
    school: CanonicalSchool,
    program: CanonicalTeam,
    body: String,
    metadata: Vec<u8>,
    interpretation: Vec<u8>,
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
