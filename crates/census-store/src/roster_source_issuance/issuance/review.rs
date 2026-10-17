use super::{ObjectRef, SemanticReview, RosterSourceReviewFieldsV1};
use super::super::{codec, inspection::RosterIssuanceInspectionV1, RosterSourceIssuanceStoreError as Error};
use census_domain::roster_affiliation_authority::LocallyCheckedRosterIssuanceV1;
use crate::StoreResult;
use serde::Serialize;

#[derive(Serialize)]
struct ProposalBasis<'a> {
    schema: &'static str,
    scope: &'a census_domain::roster_affiliation_authority::RosterAuthorityScopeV1,
    parser: &'a census_domain::roster_affiliation_authority::RosterParserClaimV1,
    capture: &'a census_domain::roster_affiliation_authority::RosterCaptureClaimV1,
    binding: &'a census_domain::roster_affiliation_authority::RosterExistingBindingClaimV1,
}

pub(super) fn proposal_basis(proposal: &LocallyCheckedRosterIssuanceV1) -> StoreResult<Vec<u8>> {
    codec::encode(&ProposalBasis { schema: "roster-source-review-proposal-basis-v1",
        scope: proposal.scope(), parser: proposal.parser(), capture: proposal.capture(), binding: proposal.binding() })
}

pub(super) fn semantic(
    inspection: &RosterIssuanceInspectionV1<'_>,
    proposal: &LocallyCheckedRosterIssuanceV1,
    fields: RosterSourceReviewFieldsV1,
    proposal_basis: ObjectRef,
    input_token: ObjectRef,
    prior_head: ObjectRef,
    qualification: ObjectRef,
    activation: ObjectRef,
) -> StoreResult<SemanticReview> {
    if fields.operation.as_str().len() > 256 || fields.operation.as_str().starts_with(super::super::OPERATION_PREFIX) {
        return Err(Error::Corrupt("review operation").into());
    }
    let acquired = chrono::DateTime::parse_from_rfc3339(proposal.capture().acquired_at())
        .map_err(|_| Error::Corrupt("review acquisition instant"))?;
    if fields.reviewed_at < acquired { return Err(Error::Corrupt("review precedes acquisition").into()); }
    Ok(SemanticReview {
        schema: "roster-source-semantic-review-v1".into(),
        provenance: "explicit-unauthenticated-local-operator".into(), fields,
        decision: "qualify-and-activate-exact-capture-representation-claims".into(), qualification, activation, store: inspection.store.clone(),
        input_token, prior_head, proposal_basis,
        scope_policy: proposal.scope().policy().clone(), year_convention: proposal.binding().year_convention().clone(),
        projection_merge_contract: proposal.binding().projection_merge_contract().clone(),
    })
}

pub(super) fn approval(basis: &super::ReviewBasis) -> StoreResult<Vec<u8>> {
    #[derive(Serialize)]
    struct Approval<'a> { schema: &'static str, reviewed_claims: &'a super::ReviewBasis }
    codec::encode(&Approval { schema: "roster-source-review-approval-v1", reviewed_claims: basis })
}
