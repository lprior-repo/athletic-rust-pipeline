use super::super::codec;
use super::{objects, ObjectRef, RosterSourceReviewDecisionV1};
use crate::StoreResult;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::roster_affiliation_authority::{
    LocallyCheckedRosterIssuanceV1, RosterAuthorityScopeV1, RosterCaptureClaimV1,
    RosterExistingBindingClaimV1, RosterParserClaimV1,
};
use serde::Serialize;

#[derive(Serialize)]
struct Qualification<'a> {
    schema: &'static str,
    authority: &'static str,
    decision: RosterSourceReviewDecisionV1,
    proposal_basis: &'a ObjectRef,
    scope: &'a RosterAuthorityScopeV1,
    parser: &'a RosterParserClaimV1,
    capture: &'a RosterCaptureClaimV1,
}

#[derive(Serialize)]
struct Activation<'a> {
    schema: &'static str,
    authority: &'static str,
    decision: RosterSourceReviewDecisionV1,
    qualification: &'a ObjectRef,
    proposal_basis: &'a ObjectRef,
    binding: &'a RosterExistingBindingClaimV1,
    temporal_scope: &'static str,
    membership_authority: &'static str,
}

pub(super) fn derive(
    proposal: &LocallyCheckedRosterIssuanceV1,
    basis: &ObjectRef,
    decision: RosterSourceReviewDecisionV1,
    objects: &mut objects::Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<(ObjectRef, ObjectRef)> {
    let qualification = codec::encode_with_source_grant(
        &Qualification {
            schema: "roster-source-qualification-reviewed-claim-v1",
            authority: "explicit-local-review-claim",
            decision,
            proposal_basis: basis,
            scope: proposal.scope(),
            parser: proposal.parser(),
            capture: proposal.capture(),
        },
        grant,
    )?;
    let qualification = objects::insert(objects, &qualification, grant)?;
    let activation = codec::encode_with_source_grant(
        &Activation {
            schema: "roster-source-single-capture-activation-reviewed-claim-v1",
            authority: "explicit-local-review-claim",
            decision,
            qualification: &qualification,
            proposal_basis: basis,
            binding: proposal.binding(),
            temporal_scope: "exact-utc-capture-day-only",
            membership_authority: "requires-independent-source-and-person-verification",
        },
        grant,
    )?;
    let activation = objects::insert(objects, &activation, grant)?;
    Ok((qualification, activation))
}

pub(super) fn require(
    proposal: &LocallyCheckedRosterIssuanceV1,
    review: &super::SemanticReview,
    objects: &objects::Objects,
) -> StoreResult<()> {
    let qualification = codec::encode(&Qualification {
        schema: "roster-source-qualification-reviewed-claim-v1",
        authority: "explicit-local-review-claim",
        decision: review.fields.decision,
        proposal_basis: &review.proposal_basis,
        scope: proposal.scope(),
        parser: proposal.parser(),
        capture: proposal.capture(),
    })?;
    let activation = codec::encode(&Activation {
        schema: "roster-source-single-capture-activation-reviewed-claim-v1",
        authority: "explicit-local-review-claim",
        decision: review.fields.decision,
        qualification: &review.qualification,
        proposal_basis: &review.proposal_basis,
        binding: proposal.binding(),
        temporal_scope: "exact-utc-capture-day-only",
        membership_authority: "requires-independent-source-and-person-verification",
    })?;
    if super::ledger::object(objects, &review.qualification)? != qualification
        || super::ledger::object(objects, &review.activation)? != activation
    {
        return Err(super::super::RosterSourceIssuanceStoreError::Corrupt(
            "qualification or activation reviewed object",
        )
        .into());
    }
    Ok(())
}
