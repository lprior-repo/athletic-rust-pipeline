use super::{inspection::RosterIssuanceInspectionV1, RosterReviewActorV1};
use census_domain::model::{ContractRefV1, StoreIdentity};
use census_domain::roster_affiliation_authority::{
    LocallyCheckedRosterIssuanceV1, RosterAuthorityTextV1,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod apply;
pub(super) mod ledger;
mod objects;
mod preflight;
mod prepare;
mod qualification;
mod reader;
mod replay;
mod review;
mod withdrawal;
pub use reader::{
    replay_frozen_roster_reviewed_claims_v1, CurrentRosterReviewedClaimsV1,
    FrozenRosterReviewedClaimsBundleV1, ReplayedRosterReviewedClaimsV1,
    ReviewedRosterSourceClaimV1, RosterFrozenReviewedClaimsReferenceV1,
};
pub use withdrawal::{
    PreparedRosterMemberWithdrawalV1, ReviewedRosterMemberWithdrawalV1,
    RosterMemberWithdrawalDecisionV1, RosterMemberWithdrawalReviewV1,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RosterSourceReviewDecisionV1 {
    QualifyAndActivateExactCaptureRepresentation,
    WithdrawExactSourceMember(Box<RosterMemberWithdrawalDecisionV1>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RosterSourceReviewFieldsV1 {
    pub operation: RosterAuthorityTextV1,
    pub actor: RosterReviewActorV1,
    pub decision: RosterSourceReviewDecisionV1,
    pub reviewed_at: DateTime<Utc>,
    pub rationale: RosterAuthorityTextV1,
}

pub struct RosterSourceReviewObjectsV1 {
    pub objects: Vec<Vec<u8>>,
    pub scope_policy: Vec<u8>,
    pub year_convention: Vec<u8>,
    pub projection_merge_contract: Vec<u8>,
}

pub struct PreparedRosterSourceReviewV1<'s> {
    inspection: RosterIssuanceInspectionV1<'s>,
    core: PreparedRosterReviewCore,
}

pub(super) struct PreparedRosterReviewCore {
    record: ReviewBasis,
    objects: BTreeMap<String, Vec<u8>>,
    approval: Vec<u8>,
    approval_digest: String,
}

impl PreparedRosterSourceReviewV1<'_> {
    pub fn approval_bytes(&self) -> &[u8] {
        &self.core.approval
    }
    pub fn approval_digest(&self) -> &str {
        &self.core.approval_digest
    }
    pub fn reviewed_proposal_bytes(&self) -> &[u8] {
        &self.core.record.proposal
    }
    pub fn source_parts(
        &mut self,
    ) -> crate::StoreResult<(
        &super::RosterSourceEvidenceV1,
        &mut census_domain::evidence_subject_attachment::AttachmentReplayGrantV1,
    )> {
        self.inspection.source_parts()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ObjectRef {
    digest: String,
    bytes: usize,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReviewBasis {
    schema: String,
    store: StoreIdentity,
    operation: RosterAuthorityTextV1,
    inspected_sequence: u64,
    evidence_generation: u64,
    derived_generation: u64,
    input_token: ObjectRef,
    prior_head: ObjectRef,
    next_ordinal: u64,
    proposal: Vec<u8>,
    review: ObjectRef,
    proposal_review_basis: ObjectRef,
    physical_inventory: ObjectRef,
    closure: Vec<ObjectRef>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReviewRecord {
    basis: ReviewBasis,
    approval_digest: String,
    approval: Vec<u8>,
    committed_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SemanticReview {
    schema: String,
    provenance: String,
    fields: RosterSourceReviewFieldsV1,
    decision: String,
    qualification: ObjectRef,
    activation: ObjectRef,
    store: StoreIdentity,
    input_token: ObjectRef,
    prior_head: ObjectRef,
    proposal_basis: ObjectRef,
    scope_policy: ContractRefV1,
    year_convention: ContractRefV1,
    projection_merge_contract: ContractRefV1,
}

fn proposal(bytes: &[u8]) -> crate::StoreResult<LocallyCheckedRosterIssuanceV1> {
    census_domain::roster_affiliation_authority::decode_roster_issuance_claim(bytes, limits()?)
        .map_err(|_| super::RosterSourceIssuanceStoreError::Corrupt("reviewed proposal").into())
}

fn limits(
) -> crate::StoreResult<census_domain::roster_affiliation_authority::RosterAuthorityLimitsV1> {
    census_domain::roster_affiliation_authority::RosterAuthorityLimitsV1::new(
        64 * 1024,
        8 * 1024 * 1024,
    )
    .map_err(|_| super::RosterSourceIssuanceStoreError::BudgetExceeded.into())
}

pub(super) const RECORD_PREFIX: &str = "roster_source_issuance/v1/reviews/";
pub(super) const OBJECT_PREFIX: &str = "roster_source_issuance/v1/objects/";
