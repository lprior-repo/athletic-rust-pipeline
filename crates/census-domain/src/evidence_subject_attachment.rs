use crate::model::{CanonicalJsonError, StoreIdentity};
use crate::public_identity_history::ReplayLimits;

mod atoms;
mod checked;
mod codec;
mod context;
mod grant;
mod inventory;
mod projection;
mod types;
mod validate;

pub use atoms::{AttachmentDigestV1, AttachmentListV1, AttachmentTextV1};
pub use checked::{
    check_attachment_command_with_grant, decode_checked_reviewed_reference_with_grant,
    CheckedAttachmentCommandV1, CheckedReviewedReferenceRecordV1,
    CheckedReviewedReferenceRecordsV1,
};
pub use codec::{
    decode_reviewed_reference, decode_reviewed_reference_with_grant, encode_reviewed_reference,
    encode_reviewed_reference_with_grant,
};
pub use context::*;
pub use grant::{AttachmentReplayGrantV1, AttachmentReplayUsageV1};
pub use inventory::calculate_reviewed_reference_inventory_with_grant;
pub use projection::{
    decode_and_replay_reviewed_references, decode_and_replay_reviewed_references_with_grant,
    replay_checked_reviewed_references_with_grant, replay_reviewed_references,
    replay_reviewed_references_with_grant, OwnedReviewedReferenceProjectionV1,
    ReviewedReferenceProjectionV1,
};
pub use types::*;

pub const EVIDENCE_SUBJECT_ATTACHMENT_POLICY_V1: &str = "retained-evidence-subject-attachment/v1";
const APPROVAL_SCHEMA: &str = "retained-evidence-subject-attachment-approval/v1";
const RECORD_SCHEMA: &str = "retained-evidence-subject-attachment-record/v1";
const INVENTORY_SCHEMA: &str = "retained-evidence-subject-attachment-inventory/v1";

type Result<T> = std::result::Result<T, EvidenceSubjectAttachmentError>;

#[derive(Debug, thiserror::Error)]
pub enum EvidenceSubjectAttachmentError {
    #[error("evidence subject attachment admission is poisoned")]
    Poisoned,
    #[error("invalid evidence subject attachment: {0}")]
    Invalid(&'static str),
    #[error("evidence subject attachment conflict: {0}")]
    Conflict(&'static str),
    #[error(
        "evidence subject attachment {resource} capacity: needed {needed}, available {available}"
    )]
    Capacity {
        resource: &'static str,
        needed: usize,
        available: usize,
    },
    #[error("evidence subject attachment arithmetic overflow")]
    Overflow,
    #[error("evidence subject attachment allocation refused")]
    Allocation,
    #[error("evidence subject attachment encoding: {0}")]
    Encoding(
        #[from]
        #[source]
        CanonicalJsonError,
    ),
    #[error("evidence subject attachment decoding: {0}")]
    Decoding(
        #[from]
        #[source]
        serde_json::Error,
    ),
}

fn limit(resource: &'static str, needed: usize, available: usize) -> Result<()> {
    if needed > available {
        return Err(EvidenceSubjectAttachmentError::Capacity {
            resource,
            needed,
            available,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
