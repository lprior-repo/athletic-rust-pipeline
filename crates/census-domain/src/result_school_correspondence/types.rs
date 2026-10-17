pub use crate::evidence_subject_attachment::{
    AttachmentDigestV1 as ResultSchoolDigestV1, AttachmentTextV1 as ResultSchoolTextV1,
};
use crate::evidence_subject_attachment::{
    AttachmentInventoryV1, AttachmentListV1, AttachmentObjectReferenceV1, AttachmentPublicHeadV1,
    AttachmentRecordReferenceV1, AttachmentTargetV1,
};
use crate::model::{
    Gender, ProfileDate, SchoolId, SchoolYear, SourceSubject, Sport, StoreIdentity,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ResultSchoolPriorCapabilityV1 {
    IdentityOnly {
        qualification: ResultSchoolDigestV1,
    },
    IdentityAndRosterSource {
        qualification: ResultSchoolDigestV1,
        roster_enrollment: ResultSchoolDigestV1,
    },
}

impl ResultSchoolPriorCapabilityV1 {
    pub fn lifecycle(&self) -> u32 {
        match self {
            Self::IdentityOnly { .. } => 7,
            Self::IdentityAndRosterSource { .. } => 8,
        }
    }
    pub fn qualification(&self) -> &ResultSchoolDigestV1 {
        match self {
            Self::IdentityOnly { qualification }
            | Self::IdentityAndRosterSource { qualification, .. } => qualification,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultSchoolStatementClaimV1 {
    pub result_root: AttachmentObjectReferenceV1,
    pub body: AttachmentObjectReferenceV1,
    pub interpretation: AttachmentObjectReferenceV1,
    pub row_locator: ResultSchoolDigestV1,
    pub row_digest: ResultSchoolDigestV1,
    pub row_offset: u64,
    pub row_length: u64,
    pub statement_offset: u64,
    pub statement_length: u64,
    pub statement_digest: ResultSchoolDigestV1,
    pub statement_text: ResultSchoolTextV1,
    pub parser_contract: AttachmentObjectReferenceV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultSchoolTargetClaimV1 {
    pub school: SchoolId,
    pub source_school: SourceSubject,
    pub provider_school_root: AttachmentObjectReferenceV1,
    pub physical_school_inventory: AttachmentObjectReferenceV1,
    pub day: ProfileDate,
    pub sport: Sport,
    pub category: Gender,
    pub year: SchoolYear,
    pub year_contract: AttachmentObjectReferenceV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultSchoolSourceClaimV1 {
    pub statement: ResultSchoolStatementClaimV1,
    pub target: ResultSchoolTargetClaimV1,
    pub person: AttachmentTargetV1,
    pub person_attachment: AttachmentRecordReferenceV1,
    pub person_inventory: AttachmentInventoryV1,
    pub evidence: AttachmentListV1<AttachmentObjectReferenceV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultSchoolInventoryV1 {
    pub version: u8,
    pub records: u64,
    pub digest: ResultSchoolDigestV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultSchoolRecordReferenceV1 {
    pub operation: ResultSchoolTextV1,
    pub digest: ResultSchoolDigestV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ResultSchoolPredecessorV1 {
    First,
    After(ResultSchoolRecordReferenceV1),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ResultSchoolActionV1 {
    Accept {
        prior: ResultSchoolPredecessorV1,
    },
    Revoke {
        prior: ResultSchoolRecordReferenceV1,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResultSchoolReviewActorV1 {
    LocalOperator,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultSchoolReviewCommandV1 {
    pub version: u8,
    pub policy: ResultSchoolTextV1,
    pub store: StoreIdentity,
    pub operation: ResultSchoolTextV1,
    pub input_token: ResultSchoolDigestV1,
    pub evidence_generation: u64,
    pub derived_generation: u64,
    pub accepted_head: AttachmentPublicHeadV1,
    pub expected_inventory: ResultSchoolInventoryV1,
    pub source: ResultSchoolSourceClaimV1,
    pub action: ResultSchoolActionV1,
    pub actor: ResultSchoolReviewActorV1,
    pub reviewed_at: DateTime<Utc>,
    pub rationale: ResultSchoolTextV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultSchoolReviewedRecordV1 {
    pub command: ResultSchoolReviewCommandV1,
    pub prepared_digest: ResultSchoolDigestV1,
    pub approval_digest: ResultSchoolDigestV1,
    pub committed_at: DateTime<Utc>,
    pub record_digest: ResultSchoolDigestV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultSchoolReceiptBindingV1 {
    pub operation: ResultSchoolTextV1,
    pub record_digest: ResultSchoolDigestV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultSchoolEnrollmentCommandV1 {
    pub version: u8,
    pub store: StoreIdentity,
    pub operation: ResultSchoolTextV1,
    pub prior: ResultSchoolPriorCapabilityV1,
    pub input_token: ResultSchoolDigestV1,
    pub evidence_generation: u64,
    pub derived_generation: u64,
    pub accepted_head: AttachmentPublicHeadV1,
    pub actor: ResultSchoolReviewActorV1,
    pub reviewed_at: DateTime<Utc>,
    pub rationale: ResultSchoolTextV1,
}
