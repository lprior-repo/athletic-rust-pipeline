pub use crate::evidence_subject_attachment::{
    AttachmentDigestV1 as ResultCalendarDigestV1, AttachmentTextV1 as ResultCalendarTextV1,
};
use crate::evidence_subject_attachment::{
    AttachmentInventoryV1, AttachmentListV1, AttachmentObjectReferenceV1, AttachmentPublicHeadV1,
    AttachmentRecordReferenceV1, AttachmentTargetV1,
};
use crate::model::{Gender, ProfileDate, SchoolYear, Sport, StoreIdentity};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultCalendarRowClaimV1 {
    pub result_root: AttachmentObjectReferenceV1,
    pub body: AttachmentObjectReferenceV1,
    pub interpretation: AttachmentObjectReferenceV1,
    pub row_locator: ResultCalendarDigestV1,
    pub row_digest: ResultCalendarDigestV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultCalendarEntryClaimV1 {
    pub documentary_root: AttachmentObjectReferenceV1,
    pub capture: ResultCalendarDigestV1,
    pub interpretation: ResultCalendarDigestV1,
    pub locator: ResultCalendarDigestV1,
    pub scope: ResultCalendarTextV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultCalendarSourceClaimV1 {
    pub result: ResultCalendarRowClaimV1,
    pub calendar: ResultCalendarEntryClaimV1,
    pub person: AttachmentTargetV1,
    pub result_attachment: AttachmentRecordReferenceV1,
    pub documentary_attachment: AttachmentRecordReferenceV1,
    pub attachment_inventory: AttachmentInventoryV1,
    pub day: ProfileDate,
    pub sport: Sport,
    pub category: Gender,
    pub year: SchoolYear,
    pub evidence: AttachmentListV1<AttachmentObjectReferenceV1>,
    pub parser_contract: AttachmentObjectReferenceV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultCalendarInventoryV1 {
    pub version: u8,
    pub records: u64,
    pub digest: ResultCalendarDigestV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultCalendarRecordReferenceV1 {
    pub operation: ResultCalendarTextV1,
    pub digest: ResultCalendarDigestV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ResultCalendarPredecessorV1 {
    First,
    After(ResultCalendarRecordReferenceV1),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ResultCalendarActionV1 {
    Accept {
        prior: ResultCalendarPredecessorV1,
    },
    Revoke {
        prior: ResultCalendarRecordReferenceV1,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResultCalendarReviewActorV1 {
    LocalOperator,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultCalendarReviewCommandV1 {
    pub version: u8,
    pub policy: ResultCalendarTextV1,
    pub store: StoreIdentity,
    pub operation: ResultCalendarTextV1,
    pub input_token: ResultCalendarDigestV1,
    pub evidence_generation: u64,
    pub derived_generation: u64,
    pub accepted_head: AttachmentPublicHeadV1,
    pub expected_inventory: ResultCalendarInventoryV1,
    pub source: ResultCalendarSourceClaimV1,
    pub action: ResultCalendarActionV1,
    pub actor: ResultCalendarReviewActorV1,
    pub reviewed_at: DateTime<Utc>,
    pub rationale: ResultCalendarTextV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultCalendarReviewedRecordV1 {
    pub command: ResultCalendarReviewCommandV1,
    pub prepared_digest: ResultCalendarDigestV1,
    pub approval_digest: ResultCalendarDigestV1,
    pub committed_at: DateTime<Utc>,
    pub record_digest: ResultCalendarDigestV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultCalendarReceiptBindingV1 {
    pub operation: ResultCalendarTextV1,
    pub record_digest: ResultCalendarDigestV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultCalendarEnrollmentCommandV1 {
    pub version: u8,
    pub store: StoreIdentity,
    pub operation: ResultCalendarTextV1,
    pub prior_result_school_enrollment: ResultCalendarDigestV1,
    pub input_token: ResultCalendarDigestV1,
    pub evidence_generation: u64,
    pub derived_generation: u64,
    pub accepted_head: AttachmentPublicHeadV1,
    pub actor: ResultCalendarReviewActorV1,
    pub reviewed_at: DateTime<Utc>,
    pub rationale: ResultCalendarTextV1,
}
