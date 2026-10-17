use census_domain::model::StoreIdentity;
use census_domain::roster_affiliation_authority::RosterAuthorityTextV1;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::StoreRevision;

mod apply;
mod inspect;
mod replay;
pub use inspect::{RosterEnrollmentInputsV1, RosterEnrollmentReviewV1};
pub(super) mod prepare;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RosterReviewActorV1 { LocalOperator }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RosterEnrollmentCommandV1 {
    pub version: u8,
    pub store: StoreIdentity,
    pub operation: RosterAuthorityTextV1,
    pub actor: RosterReviewActorV1,
    pub reviewed_at: DateTime<Utc>,
    pub rationale: RosterAuthorityTextV1,
    pub evidence_generation: u64,
    pub derived_generation: u64,
    pub input_token_digest: RosterAuthorityTextV1,
}

pub struct PreparedRosterEnrollmentV1<'s> {
    revision: StoreRevision<'s>,
    command: RosterEnrollmentCommandV1,
    approval: Vec<u8>,
    approval_digest: String,
}

impl PreparedRosterEnrollmentV1<'_> {
    pub fn approval_bytes(&self) -> &[u8] { &self.approval }
    pub fn approval_digest(&self) -> &str { &self.approval_digest }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EnrollmentRecord {
    pub(super) schema: String,
    pub(super) command: RosterEnrollmentCommandV1,
    pub(super) approval_digest: String,
    pub(super) committed_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AuthorityHead {
    pub(super) version: u8,
    pub(super) ordinal: u64,
    pub(super) digest: String,
}

#[derive(Serialize)]
struct Approval<'a> {
    schema: &'static str,
    actor: RosterReviewActorV1,
    prepared: &'a RosterEnrollmentCommandV1,
}

fn approval(command: &RosterEnrollmentCommandV1) -> crate::StoreResult<Vec<u8>> {
    super::codec::encode(&Approval {
        schema: "roster-source-enrollment-approval-v1",
        actor: command.actor,
        prepared: command,
    })
}
