use super::*;
use std::ops::Deref;
use std::sync::Arc;

mod command;
mod record;
mod records;

pub use command::{check_attachment_command_with_grant, CheckedAttachmentCommandV1};
pub use record::{decode_checked_reviewed_reference_with_grant, CheckedReviewedReferenceRecordV1};
pub use records::CheckedReviewedReferenceRecordsV1;

#[cfg(test)]
mod tests;
