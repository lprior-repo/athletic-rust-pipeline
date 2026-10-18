use super::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Enrollment {
    pub(super) schema: String,
    pub(super) command: ResultSchoolEnrollmentCommandV1,
    pub(super) initial_inventory: ResultSchoolInventoryV1,
    pub(super) approval_digest: ResultSchoolDigestV1,
    pub(super) committed_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) enum PriorProof {
    IdentityOnly,
    IdentityAndRosterSource {
        #[serde(with = "super::codec::json_bytes")]
        enrollment: Vec<u8>,
        #[serde(with = "super::codec::json_bytes")]
        receipt: Vec<u8>,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Proof {
    pub(super) schema: String,
    #[serde(with = "super::codec::json_bytes")]
    pub(super) enrollment: Vec<u8>,
    #[serde(with = "super::codec::json_bytes")]
    pub(super) receipt: Vec<u8>,
    #[serde(with = "super::codec::json_bytes")]
    pub(super) qualification: Vec<u8>,
    pub(super) prior: PriorProof,
}

#[derive(Serialize)]
struct Approval<'a> {
    schema: &'static str,
    command: &'a ResultSchoolEnrollmentCommandV1,
}

pub(super) fn enrollment_approval(
    command: &ResultSchoolEnrollmentCommandV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    validate_enrollment_command(command)?;
    codec::encode(
        &Approval {
            schema: "result-school-enrollment-approval-v1",
            command,
        },
        grant,
    )
}

pub(super) fn initial(
    command: &ResultSchoolEnrollmentCommandV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<ResultSchoolInventoryV1> {
    let basis = result_school_digest_v1(command, grant).map_err(Error::Domain)?;
    initial_result_school_inventory_v1(&command.store, &basis, grant)
        .map_err(|error| Error::Domain(error).into())
}

pub(super) fn enrollment_operation(command: &ResultSchoolEnrollmentCommandV1) -> String {
    format!("{OPERATIONS}{}", command.operation.as_str())
}
pub(super) fn issuance_operation(command: &ResultSchoolReviewCommandV1) -> String {
    format!("{OPERATIONS}{}", command.operation.as_str())
}

pub(super) fn validate_enrollment_command(
    command: &ResultSchoolEnrollmentCommandV1,
) -> StoreResult<()> {
    if command.version != 1
        || command.operation.as_str().len() > 256
        || command.operation.as_str().starts_with(NAME)
    {
        return Err(Error::Corrupt("enrollment command").into());
    }
    Ok(())
}

pub(super) fn validate_review_command(command: &ResultSchoolReviewCommandV1) -> StoreResult<()> {
    if command.operation.as_str().starts_with(NAME) {
        return Err(Error::Corrupt("review operation namespace").into());
    }
    Ok(())
}

pub(super) fn review_approval(
    command: &ResultSchoolReviewCommandV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    result_school_approval_bytes_v1(command, grant).map_err(|error| Error::Domain(error).into())
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AuthorityHead {
    pub(super) schema: String,
    pub(super) inventory: ResultSchoolInventoryV1,
}

pub(super) const HEAD: &str = "result_school_correspondence/v1/head";
pub(super) const RECORDS: &str = "result_school_correspondence/v1/record/";

pub(super) fn record_key(ordinal: usize) -> StoreResult<String> {
    if ordinal >= MAX_RESULT_SCHOOL_RECORDS {
        return Err(Error::BudgetExceeded.into());
    }
    Ok(format!("{RECORDS}{ordinal:08}"))
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RecordEnvelope {
    pub(super) schema: String,
    pub(super) record: ResultSchoolReviewedRecordV1,
    pub(super) context: Vec<super::source::Material>,
}
