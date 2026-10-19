use super::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Enrollment {
    pub(super) schema: String,
    pub(super) command: ResultCalendarEnrollmentCommandV1,
    pub(super) prior_school: Box<RawValue>,
    pub(super) initial_inventory: ResultCalendarInventoryV1,
    pub(super) approval_digest: ResultCalendarDigestV1,
    pub(super) committed_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Proof {
    pub(super) schema: String,
    pub(super) enrollment: Box<RawValue>,
    pub(super) receipt: Box<RawValue>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AuthorityHead {
    pub(super) schema: String,
    pub(super) inventory: ResultCalendarInventoryV1,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RecordEnvelope {
    pub(super) schema: String,
    pub(super) record: ResultCalendarReviewedRecordV1,
    pub(super) context: Vec<super::source::Material>,
}

pub(super) const HEAD: &str = "result_calendar_correspondence/v1/head";
pub(super) const RECORDS: &str = "result_calendar_correspondence/v1/record/";

pub(super) fn record_key(ordinal: usize) -> StoreResult<String> {
    if ordinal >= MAX_RESULT_CALENDAR_RECORDS {
        return Err(Error::BudgetExceeded.into());
    }
    Ok(format!("{RECORDS}{ordinal:08}"))
}

#[derive(Serialize)]
struct Approval<'a> {
    schema: &'static str,
    command: &'a ResultCalendarEnrollmentCommandV1,
}

pub(super) fn enrollment_approval(
    command: &ResultCalendarEnrollmentCommandV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    validate_enrollment_command(command)?;
    codec::encode(
        &Approval {
            schema: "result-calendar-enrollment-approval-v1",
            command,
        },
        grant,
    )
}

pub(super) fn initial(
    command: &ResultCalendarEnrollmentCommandV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<ResultCalendarInventoryV1> {
    let basis = result_calendar_enrollment_basis_v1(command, grant).map_err(Error::Domain)?;
    initial_result_calendar_inventory_v1(&command.store, &basis, grant)
        .map_err(|error| Error::Domain(error).into())
}

pub(super) fn enrollment_operation(command: &ResultCalendarEnrollmentCommandV1) -> String {
    format!("{OPERATIONS}{}", command.operation.as_str())
}

pub(super) fn issuance_operation(command: &ResultCalendarReviewCommandV1) -> String {
    format!("{OPERATIONS}{}", command.operation.as_str())
}

pub(super) fn validate_enrollment_command(
    command: &ResultCalendarEnrollmentCommandV1,
) -> StoreResult<()> {
    if command.version != 1
        || command.operation.as_str().len() > 256
        || command.operation.as_str().starts_with(NAME)
    {
        return Err(Error::Corrupt("enrollment command").into());
    }
    Ok(())
}

pub(super) fn validate_review_command(command: &ResultCalendarReviewCommandV1) -> StoreResult<()> {
    if command.operation.as_str().starts_with(NAME) {
        return Err(Error::Corrupt("review operation namespace").into());
    }
    Ok(())
}

pub(super) fn review_approval(
    checked: &CheckedResultCalendarCommandV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    result_calendar_approval_bytes_v1(checked, grant).map_err(|error| Error::Domain(error).into())
}
