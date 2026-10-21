use crate::evidence_subject_attachment::{AttachmentReplayGrantV1, EvidenceSubjectAttachmentError};
use serde::Serialize;
mod codec;
use codec::{admit, encode, with_grant};
mod parser;
mod replay;
mod selection;
mod types;
mod validate;
pub use parser::{ResultCalendarSourceParserV1, RESULT_CALENDAR_TFRRS_RONCALLI_PARSER_CONTRACT_V1};
pub use replay::{replay_result_calendar_records_v1, ResultCalendarProjectionV1};
pub use selection::{
    RoncalliSeasonEventProofContextV1, RoncalliSeasonEventSelectionV1,
    RONCALLI_SEASON_EVENT_INTERPRETATION_V1,
};
pub use types::*;

pub const RESULT_CALENDAR_POLICY_V1: &str = "reviewed_exact_result_calendar_day_correspondence_v1";
pub const RESULT_CALENDAR_PARSER_CONTRACT_V1: &[u8] = br#"{"version":1,"contract":"reviewed-original-result-calendar-day-v1","authority":["scholastic-context","exact-original-day-in-season"],"full-window":"unasserted","cohort":"unasserted","school-identity":"unasserted","ncaa":"unasserted","raw-labels":"unchanged"}"#;
pub const MAX_RESULT_CALENDAR_RECORD_BYTES: usize = 65_536;
pub const MAX_RESULT_CALENDAR_RECORDS: usize = 256;
pub const MAX_RESULT_CALENDAR_OBJECTS: usize = 32;
type Result<T, E = ResultCalendarCorrespondenceError> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum ResultCalendarCorrespondenceError {
    #[error("invalid result calendar correspondence: {0}")]
    Invalid(&'static str),
    #[error("result calendar correspondence capacity exceeded")]
    Capacity,
    #[error("result calendar correspondence predecessor conflict")]
    PredecessorConflict,
    #[error("result calendar correspondence inventory differs")]
    InventoryMismatch,
    #[error(transparent)]
    Grant(#[from] EvidenceSubjectAttachmentError),
    #[error(transparent)]
    Encoding(#[from] crate::model::CanonicalJsonError),
    #[error(transparent)]
    Decoding(#[from] serde_json::Error),
}

pub struct CheckedResultCalendarCommandV1 {
    command: ResultCalendarReviewCommandV1,
    continuation: crate::evidence_subject_attachment::AttachmentReplayContinuationV1,
}

impl CheckedResultCalendarCommandV1 {
    pub fn check(
        command: ResultCalendarReviewCommandV1,
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
    ) -> Result<&ResultCalendarReviewCommandV1> {
        self.continuation.require(grant)?;
        Ok(&self.command)
    }

    pub fn into_command(
        self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> Result<ResultCalendarReviewCommandV1> {
        self.continuation.require(grant)?;
        Ok(self.command)
    }
}

fn result_calendar_digest_v1(
    value: &(impl Serialize + ?Sized),
    grant: &mut AttachmentReplayGrantV1,
) -> Result<ResultCalendarDigestV1> {
    with_grant(grant, |grant| codec::digest(value, grant))
}

pub fn result_calendar_enrollment_basis_v1(
    command: &ResultCalendarEnrollmentCommandV1,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<ResultCalendarDigestV1> {
    with_grant(grant, |grant| {
        if command.version != 1 || command.operation.as_str().len() > 256 {
            return Err(ResultCalendarCorrespondenceError::Invalid(
                "enrollment command",
            ));
        }
        result_calendar_digest_v1(command, grant)
    })
}

pub fn decode_result_calendar_record_v1(
    bytes: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> Result<ResultCalendarReviewedRecordV1> {
    with_grant(grant, |grant| {
        if bytes.len() > MAX_RESULT_CALENDAR_RECORD_BYTES {
            return Err(ResultCalendarCorrespondenceError::Capacity);
        }
        grant.charge_bytes(bytes.len())?;
        grant.charge_steps(1)?;
        let record: ResultCalendarReviewedRecordV1 = serde_json::from_slice(bytes)?;
        codec::require_canonical(&record, bytes, grant)?;
        validate::record(&record, grant)?;
        Ok(record)
    })
}

pub fn result_calendar_approval_bytes_v1(
    checked: &CheckedResultCalendarCommandV1,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<Vec<u8>> {
    with_grant(grant, |grant| {
        let command = checked.command(grant)?;
        encode(
            &(
                "result-calendar-correspondence-approval-v1",
                command.actor,
                command,
            ),
            grant,
        )
    })
}

fn approval_digest(
    command: &ResultCalendarReviewCommandV1,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<ResultCalendarDigestV1> {
    result_calendar_digest_v1(
        &(
            "result-calendar-correspondence-approval-v1",
            command.actor,
            command,
        ),
        grant,
    )
}

pub fn prepare_result_calendar_record_v1(
    command: CheckedResultCalendarCommandV1,
    committed_at: chrono::DateTime<chrono::Utc>,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<ResultCalendarReviewedRecordV1> {
    with_grant(grant, |grant| {
        let command = command.into_command(grant)?;
        if committed_at < command.reviewed_at {
            return Err(ResultCalendarCorrespondenceError::Invalid(
                "record digest or time",
            ));
        }
        let prepared_digest = result_calendar_digest_v1(&command, grant)?;
        let approval_digest = approval_digest(&command, grant)?;
        let record_digest = result_calendar_digest_v1(
            &(
                "result-calendar-reviewed-record-v1",
                &command,
                &prepared_digest,
                &approval_digest,
                committed_at,
            ),
            grant,
        )?;
        Ok(ResultCalendarReviewedRecordV1 {
            command,
            prepared_digest,
            approval_digest,
            committed_at,
            record_digest,
        })
    })
}

pub fn initial_result_calendar_inventory_v1(
    store: &crate::model::StoreIdentity,
    basis: &ResultCalendarDigestV1,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<ResultCalendarInventoryV1> {
    with_grant(grant, |grant| {
        Ok(ResultCalendarInventoryV1 {
            version: 1,
            records: 0,
            digest: result_calendar_digest_v1(
                &("result-calendar-empty-inventory-v1", store, basis),
                grant,
            )?,
        })
    })
}

pub fn next_result_calendar_inventory_v1(
    previous: &ResultCalendarInventoryV1,
    record: &ResultCalendarReviewedRecordV1,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<ResultCalendarInventoryV1> {
    with_grant(grant, |grant| {
        if previous.version != 1
            || previous.records
                >= u64::try_from(MAX_RESULT_CALENDAR_RECORDS)
                    .map_err(|_| ResultCalendarCorrespondenceError::Capacity)?
        {
            return Err(ResultCalendarCorrespondenceError::Capacity);
        }
        Ok(ResultCalendarInventoryV1 {
            version: 1,
            records: previous
                .records
                .checked_add(1)
                .ok_or(ResultCalendarCorrespondenceError::Capacity)?,
            digest: result_calendar_digest_v1(
                &(
                    "result-calendar-inventory-step-v1",
                    previous,
                    &record.record_digest,
                ),
                grant,
            )?,
        })
    })
}

#[cfg(test)]
mod tests;
