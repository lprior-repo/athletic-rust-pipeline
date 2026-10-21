use super::*;
use crate::participation_evidence::invalid;

pub fn assess_retained_roster_source<'a>(
    _evidence: &'a RosterSourceEvidenceV1,
    _retained_parser_manifest: &[u8],
    _retained_contracts: &super::super::policies::RosterInterpretationContractsV1<'_>,
    _proposal: &LocallyCheckedRosterIssuanceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<RosterSourceEvidenceAssessmentV1<'a>> {
    grant.poison();
    Err(invalid("unsupported retained roster source v1").into())
}

pub fn assess_retained_roster_source_v2<'a>(
    _evidence: &'a RosterSourceEvidenceV1,
    _retained_parser_manifest: &[u8],
    _retained_contracts: &super::super::policies::RosterInterpretationContractsV1<'_>,
    _proposal: &LocallyCheckedRosterIssuanceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<RosterSourceEvidenceAssessmentV1<'a>> {
    grant.poison();
    Err(invalid("unsupported retained roster source v2").into())
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
    )
}

fn assess_version<'a>(
    evidence: &'a RosterSourceEvidenceV1,
    retained_parser_manifest: &[u8],
    retained_contracts: &super::super::policies::RosterInterpretationContractsV1<'_>,
    proposal: &LocallyCheckedRosterIssuanceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<RosterSourceEvidenceAssessmentV1<'a>> {
    evidence.require_original_grant(grant)?;
    let result = assess(
        evidence,
        retained_parser_manifest,
        retained_contracts,
        proposal,
        grant,
    );
    if result.is_err() {
        grant.poison();
    }
    result
}
