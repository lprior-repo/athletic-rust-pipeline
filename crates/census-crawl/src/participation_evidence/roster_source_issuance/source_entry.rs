use super::*;

pub fn assess_retained_roster_source<'a>(
    evidence: &'a RosterSourceEvidenceV1,
    retained_parser_manifest: &[u8],
    retained_contracts: &super::super::policies::RosterInterpretationContractsV1<'_>,
    proposal: &LocallyCheckedRosterIssuanceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<RosterSourceEvidenceAssessmentV1<'a>> {
    assess_version(
        evidence,
        retained_parser_manifest,
        retained_contracts,
        proposal,
        grant,
        false,
    )
}

pub fn assess_retained_roster_source_v2<'a>(
    evidence: &'a RosterSourceEvidenceV1,
    retained_parser_manifest: &[u8],
    retained_contracts: &super::super::policies::RosterInterpretationContractsV1<'_>,
    proposal: &LocallyCheckedRosterIssuanceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<RosterSourceEvidenceAssessmentV1<'a>> {
    assess_version(
        evidence,
        retained_parser_manifest,
        retained_contracts,
        proposal,
        grant,
        true,
    )
}

fn assess_version<'a>(
    evidence: &'a RosterSourceEvidenceV1,
    retained_parser_manifest: &[u8],
    retained_contracts: &super::super::policies::RosterInterpretationContractsV1<'_>,
    proposal: &LocallyCheckedRosterIssuanceV1,
    grant: &mut AttachmentReplayGrantV1,
    semantic_inventory: bool,
) -> CrawlResult<RosterSourceEvidenceAssessmentV1<'a>> {
    evidence.require_original_grant(grant)?;
    let result = assess(
        evidence,
        retained_parser_manifest,
        retained_contracts,
        proposal,
        grant,
        semantic_inventory,
    );
    if result.is_err() {
        grant.poison();
    }
    result
}
