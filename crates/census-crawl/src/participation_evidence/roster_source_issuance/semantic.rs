use super::{
    assess_existing_roster_binding, assess_retained_roster_source,
    policies::RosterInterpretationContractsV1, RosterSourceEvidenceAssessmentV1,
};
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{CanonicalSchool, CanonicalTeam};
use census_domain::roster_affiliation_authority::LocallyCheckedRosterIssuanceV1;
use census_store::{ReviewedRosterSourceClaimV1, StoreResult};

pub struct ReviewedRosterSourceAssessmentV1<'a> {
    claim: &'a ReviewedRosterSourceClaimV1,
    source: RosterSourceEvidenceAssessmentV1<'a>,
    school: CanonicalSchool,
    program: CanonicalTeam,
}

pub fn assess_reviewed_roster_source_v1<'a>(
    claim: &'a ReviewedRosterSourceClaimV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<ReviewedRosterSourceAssessmentV1<'a>> {
    let result = assess(claim, grant);
    if result.is_err() {
        grant.poison();
    }
    result
}

fn assess<'a>(
    claim: &'a ReviewedRosterSourceClaimV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<ReviewedRosterSourceAssessmentV1<'a>> {
    let proposal = claim.proposal(grant)?;
    let parser = claim.object(proposal.parser().artifact().digest().as_str(), grant)?;
    let contracts = RosterInterpretationContractsV1 {
        scope: claim.object(proposal.scope().policy().digest(), grant)?,
        year: claim.object(proposal.binding().year_convention().digest(), grant)?,
        projection_merge: claim.object(
            proposal.binding().projection_merge_contract().digest(),
            grant,
        )?,
    };
    let interpretation =
        claim.object(proposal.capture().interpretation().digest().as_str(), grant)?;
    grant
        .charge_bytes(interpretation.len())
        .map_err(super::source::domain)?;
    grant.charge_steps(1).map_err(super::source::domain)?;
    #[derive(serde::Deserialize)]
    struct Version<'v> {
        schema: &'v str,
    }
    let version: Version<'_> =
        serde_json::from_slice(interpretation).map_err(crate::participation_evidence::json)?;
    let source = match version.schema {
        "roster-retained-capture-interpretation-v1" => assess_retained_roster_source(
            claim.evidence(grant)?,
            parser,
            &contracts,
            proposal,
            grant,
        )?,
        "roster-retained-capture-interpretation-v2" => {
            super::source::assess_retained_roster_source_v2(
                claim.evidence(grant)?,
                parser,
                &contracts,
                proposal,
                grant,
            )?
        }
        _ => {
            return Err(crate::participation_evidence::invalid(
                "unsupported reviewed roster interpretation",
            )
            .into())
        }
    };
    let binding = assess_existing_roster_binding(&source, grant)?;
    super::source::budget::charge(binding.school(grant)?, grant)?;
    let school = binding.school(grant)?.clone();
    super::source::budget::charge(binding.program(grant)?, grant)?;
    let program = binding.program(grant)?.clone();
    claim.qualification_object(grant)?;
    claim.activation_object(grant)?;
    source.require_original_grant(grant)?;
    Ok(ReviewedRosterSourceAssessmentV1 {
        claim,
        source,
        school,
        program,
    })
}

impl ReviewedRosterSourceAssessmentV1<'_> {
    pub fn require_original_grant(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<()> {
        self.source.require_original_grant(grant)
    }

    pub fn source(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<&RosterSourceEvidenceAssessmentV1<'_>> {
        self.require_original_grant(grant)?;
        Ok(&self.source)
    }

    pub fn proposal(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<&LocallyCheckedRosterIssuanceV1> {
        self.claim.proposal(grant)
    }

    pub fn school(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<&CanonicalSchool> {
        self.require_original_grant(grant)?;
        Ok(&self.school)
    }

    pub fn program(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<&CanonicalTeam> {
        self.require_original_grant(grant)?;
        Ok(&self.program)
    }

    pub fn qualification_object(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<(&str, &[u8])> {
        self.require_original_grant(grant)?;
        self.claim.qualification_object(grant)
    }

    pub fn activation_object(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<(&str, &[u8])> {
        self.require_original_grant(grant)?;
        self.claim.activation_object(grant)
    }

    pub fn binding_semantics(&self) -> &'static str {
        "computed-unique-existing-provider-match"
    }
}
