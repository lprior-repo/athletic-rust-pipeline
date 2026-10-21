use crate::evidence_subject_attachment::{AttachmentReplayGrantV1, EvidenceSubjectAttachmentError};
use serde::Serialize;
mod codec;
mod parser;
use codec::{encode, with_grant};
pub use parser::{ResultSchoolSourceParserV1, RESULT_SCHOOL_TFRRS_PARSER_CONTRACT_V1};

mod replay;
mod types;
mod validate;
pub use replay::{replay_result_school_records_v1, ResultSchoolProjectionV1};
pub use types::*;

pub const RESULT_SCHOOL_POLICY_V1: &str = "reviewed_exact_result_school_correspondence_v1";
pub const RESULT_SCHOOL_PARSER_CONTRACT_V1: &[u8] = br#"{"version":1,"contract":"result-school-original-cell-v1","row_interpretation":"retained-milesplit-html-result-row/v1","statement_interpretation":"retained-milesplit-original-school-statement/v1","ownership":"unique-untransformed-original-school-cell","identity_minting":"forbidden"}"#;
pub const RESULT_SCHOOL_YEAR_CONTRACT_V1: &[u8] = br#"{"version":1,"contract":"result-school-existing-program-key-year-v1","implementation":"SchoolYear::from_date","date":"performance-source-day","authority":"existing-program-key-only","academic-season-coverage":"unasserted","eligibility-window":"unasserted"}"#;
pub const MAX_RESULT_SCHOOL_RECORD_BYTES: usize = 65_536;
pub const MAX_RESULT_SCHOOL_RECORDS: usize = 256;
pub const MAX_RESULT_SCHOOL_OBJECTS: usize = 32;

type Result<T> = std::result::Result<T, ResultSchoolCorrespondenceError>;

#[derive(Debug, thiserror::Error)]
pub enum ResultSchoolCorrespondenceError {
    #[error("invalid result school correspondence: {0}")]
    Invalid(&'static str),
    #[error("result school correspondence capacity exceeded")]
    Capacity,
    #[error("result school correspondence predecessor conflict")]
    PredecessorConflict,
    #[error("result school correspondence inventory differs")]
    InventoryMismatch,
    #[error(transparent)]
    Grant(#[from] EvidenceSubjectAttachmentError),
    #[error(transparent)]
    Encoding(#[from] crate::model::CanonicalJsonError),
    #[error(transparent)]
    Decoding(#[from] serde_json::Error),
}

pub struct CheckedResultSchoolCommandV1 {
    command: ResultSchoolReviewCommandV1,
    continuation: crate::evidence_subject_attachment::AttachmentReplayContinuationV1,
}

impl CheckedResultSchoolCommandV1 {
    pub fn check(
        command: ResultSchoolReviewCommandV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> Result<Self> {
        with_grant(grant, |grant| {
            validate::command(&command, grant)?;
            Ok(Self {
                command,
                continuation: grant.checkpoint()?,
            })
        })
    }
    pub fn command(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> Result<&ResultSchoolReviewCommandV1> {
        self.continuation.require(grant)?;
        Ok(&self.command)
    }
    pub fn into_command(
        self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> Result<ResultSchoolReviewCommandV1> {
        self.continuation.require(grant)?;
        Ok(self.command)
    }
}

pub fn result_school_digest_v1(
    value: &(impl Serialize + ?Sized),
    grant: &mut AttachmentReplayGrantV1,
) -> Result<ResultSchoolDigestV1> {
    with_grant(grant, |grant| {
        let bytes = encode(value, grant)?;
        use sha2::{Digest, Sha256};
        Ok(ResultSchoolDigestV1::parse(&format!(
            "{:x}",
            Sha256::digest(bytes)
        ))?)
    })
}

pub fn decode_result_school_record_v1(
    bytes: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> Result<ResultSchoolReviewedRecordV1> {
    with_grant(grant, |grant| {
        if bytes.len() > MAX_RESULT_SCHOOL_RECORD_BYTES {
            return Err(ResultSchoolCorrespondenceError::Capacity);
        }
        grant.charge_bytes(bytes.len())?;
        grant.charge_steps(1)?;
        let record: ResultSchoolReviewedRecordV1 = serde_json::from_slice(bytes)?;
        if encode(&record, grant)? != bytes {
            return Err(ResultSchoolCorrespondenceError::Invalid(
                "noncanonical record",
            ));
        }
        validate::record(&record, grant)?;
        Ok(record)
    })
}

pub fn result_school_approval_bytes_v1(
    command: &ResultSchoolReviewCommandV1,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<Vec<u8>> {
    with_grant(grant, |grant| {
        encode(
            &(
                "result-school-correspondence-approval-v1",
                command.actor,
                command,
            ),
            grant,
        )
    })
}

pub fn prepare_result_school_record_v1(
    command: CheckedResultSchoolCommandV1,
    committed_at: chrono::DateTime<chrono::Utc>,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<ResultSchoolReviewedRecordV1> {
    with_grant(grant, |grant| {
        let command = command.into_command(grant)?;
        if committed_at < command.reviewed_at {
            return Err(ResultSchoolCorrespondenceError::Invalid(
                "record digest or time",
            ));
        }
        let prepared_digest = result_school_digest_v1(&command, grant)?;
        let approval_digest = digest_bytes(&result_school_approval_bytes_v1(&command, grant)?)?;
        let record_digest = result_school_digest_v1(
            &(
                "result-school-reviewed-record-v1",
                &command,
                &prepared_digest,
                &approval_digest,
                committed_at,
            ),
            grant,
        )?;
        Ok(ResultSchoolReviewedRecordV1 {
            command,
            prepared_digest,
            approval_digest,
            committed_at,
            record_digest,
        })
    })
}

pub fn initial_result_school_inventory_v1(
    store: &crate::model::StoreIdentity,
    basis: &ResultSchoolDigestV1,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<ResultSchoolInventoryV1> {
    with_grant(grant, |grant| {
        Ok(ResultSchoolInventoryV1 {
            version: 1,
            records: 0,
            digest: result_school_digest_v1(
                &("result-school-empty-inventory-v1", store, basis),
                grant,
            )?,
        })
    })
}

pub fn next_result_school_inventory_v1(
    previous: &ResultSchoolInventoryV1,
    record: &ResultSchoolReviewedRecordV1,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<ResultSchoolInventoryV1> {
    with_grant(grant, |grant| {
        Ok(ResultSchoolInventoryV1 {
            version: 1,
            records: previous
                .records
                .checked_add(1)
                .ok_or(ResultSchoolCorrespondenceError::Capacity)?,
            digest: result_school_digest_v1(
                &(
                    "result-school-inventory-step-v1",
                    previous,
                    &record.record_digest,
                ),
                grant,
            )?,
        })
    })
}

fn digest_bytes(bytes: &[u8]) -> Result<ResultSchoolDigestV1> {
    use sha2::{Digest, Sha256};
    Ok(ResultSchoolDigestV1::parse(&format!(
        "{:x}",
        Sha256::digest(bytes)
    ))?)
}

#[cfg(test)]
mod tests;
