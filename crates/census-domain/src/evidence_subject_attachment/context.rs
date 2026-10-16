use super::{AttachmentDigestV1, AttachmentListV1, AttachmentTextV1};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttachmentObjectReferenceV1 {
    pub digest: AttachmentDigestV1,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetainedDocumentSubjectReferenceV1 {
    pub namespace: AttachmentTextV1,
    pub native_id: Option<AttachmentTextV1>,
    pub capture: AttachmentDigestV1,
    pub body: AttachmentDigestV1,
    pub interpretation: AttachmentDigestV1,
    pub locator: AttachmentDigestV1,
    pub subject: AttachmentDigestV1,
    pub context: AttachmentDigestV1,
    pub role: AttachmentSubjectRoleV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttachmentSubjectRoleV1 {
    AthleteStatus,
    ResultRow,
    CorroboratingRoster,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttachmentFactDispositionV1 {
    SupportsAttachment,
    ContradictsAttachment,
    ContextOnly,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewedAttachmentFactV1 {
    pub fact: AttachmentDigestV1,
    pub subject: AttachmentDigestV1,
    pub publisher_lineage: AttachmentDigestV1,
    pub disposition: AttachmentFactDispositionV1,
    pub rationale: AttachmentTextV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttachmentCorroborationBasisV1 {
    ExplicitOriginalCrossReference,
    IndependentPersonSpecificContext,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttachmentCorroborationPairV1 {
    pub subject_fact: AttachmentDigestV1,
    pub accepted_target_fact: AttachmentDigestV1,
    pub basis: AttachmentCorroborationBasisV1,
    pub explanation: AttachmentTextV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewedAttachmentContextV1 {
    pub objects: AttachmentListV1<AttachmentObjectReferenceV1>,
    pub facts: AttachmentListV1<ReviewedAttachmentFactV1>,
    pub corroboration: AttachmentListV1<AttachmentCorroborationPairV1>,
    pub complete_scope_records: AttachmentListV1<AttachmentDigestV1>,
    pub accepted_target_subjects: AttachmentListV1<AttachmentDigestV1>,
    pub accepted_target_witness: AttachmentDigestV1,
}
