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
        InterpretationVersion::OriginalV1,
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
        InterpretationVersion::FullV2,
    )
}

pub fn assess_retained_roster_source_v3<'a>(
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
        InterpretationVersion::RowLocalV3,
    )
}

fn assess_version<'a>(
    evidence: &'a RosterSourceEvidenceV1,
    retained_parser_manifest: &[u8],
    retained_contracts: &super::super::policies::RosterInterpretationContractsV1<'_>,
    proposal: &LocallyCheckedRosterIssuanceV1,
    grant: &mut AttachmentReplayGrantV1,
    version: InterpretationVersion,
) -> CrawlResult<RosterSourceEvidenceAssessmentV1<'a>> {
    evidence.require_original_grant(grant)?;
    let result = assess(
        evidence,
        retained_parser_manifest,
        retained_contracts,
        proposal,
        grant,
        version,
    );
    if result.is_err() {
        grant.poison();
    }
    result
}
