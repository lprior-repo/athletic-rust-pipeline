use super::*;
use crate::model::AthleteCandidateId;
use crate::public_identity_history::PublicAthleteId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttachmentPublicHeadV1 {
    pub sequence: u64,
    pub digest: AttachmentDigestV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttachmentInventoryV1 {
    pub version: u8,
    pub records: u64,
    pub digest: AttachmentDigestV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttachmentTargetV1 {
    #[serde(deserialize_with = "atoms::public_id")]
    pub public_id: PublicAthleteId,
    #[serde(deserialize_with = "atoms::candidate")]
    pub canonical_candidate: AthleteCandidateId,
    pub accepted_group_digest: AttachmentDigestV1,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttachmentRecordReferenceV1 {
    pub operation: AttachmentTextV1,
    pub digest: AttachmentDigestV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum AttachmentPredecessorV1 {
    First,
    After(AttachmentRecordReferenceV1),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum AttachmentActionV1 {
    Attach { prior: AttachmentPredecessorV1 },
    Revoke { prior: AttachmentRecordReferenceV1 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttachmentReviewActorV1 {
    LocalOperator,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetainedEvidenceSubjectAttachmentCommandV1 {
    pub version: u8,
    pub policy: AttachmentTextV1,
    #[serde(deserialize_with = "atoms::store")]
    pub store: StoreIdentity,
    pub operation: AttachmentTextV1,
    pub expected_public_head: AttachmentPublicHeadV1,
    pub expected_evidence_generation: u64,
    pub expected_derived_generation: u64,
    pub expected_attachment_inventory: AttachmentInventoryV1,
    pub target: AttachmentTargetV1,
    pub subjects: AttachmentListV1<RetainedDocumentSubjectReferenceV1>,
    pub action: AttachmentActionV1,
    pub reviewed_context: ReviewedAttachmentContextV1,
    pub actor: AttachmentReviewActorV1,
    pub reviewed_at: DateTime<Utc>,
    pub rationale: AttachmentTextV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewedReferenceRecordV1 {
    pub command: RetainedEvidenceSubjectAttachmentCommandV1,
    pub prepared_digest: AttachmentDigestV1,
    pub approval_digest: AttachmentDigestV1,
    pub committed_at: DateTime<Utc>,
    pub record_digest: AttachmentDigestV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttachmentReceiptBindingV1 {
    pub operation: AttachmentTextV1,
    pub record_digest: AttachmentDigestV1,
}
