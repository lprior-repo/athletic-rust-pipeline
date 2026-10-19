use crate::{StoreError, StoreResult};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::result_calendar_correspondence::*;

mod codec;
mod current;
mod enrollment;
mod issuance;
mod proof;
mod read;
mod source;
mod wire;

pub use current::CurrentResultCalendarCorrespondencesV1;
pub use enrollment::{
    PreparedResultCalendarEnrollmentV1, ResultCalendarEnrollmentInputsV1,
    ResultCalendarEnrollmentReviewV1,
};
pub use issuance::{
    PreparedResultCalendarCorrespondenceV1, ResultCalendarCorrespondenceInspectionV1,
    ResultCalendarCorrespondencePreparationV1,
};
pub use proof::{
    replay_result_calendar_enrollment_proof_v1, replay_result_calendar_source_proof_v1,
    require_result_calendar_school_predecessor_v1, ReplayedResultCalendarCorrespondencesV1,
    ResultCalendarEnrollmentProofV1, ResultCalendarSourceProofV1,
};
pub(crate) use read::{preserved_prior_capability, validate_format};

pub const RESULT_CALENDAR_CORRESPONDENCE_LIFECYCLE: u32 = 10;
const LIFECYCLE: u32 = RESULT_CALENDAR_CORRESPONDENCE_LIFECYCLE;
const NAME: &str = "result_calendar_correspondence";
const ENROLLMENT: &str = "result_calendar_correspondence/v1/enrollment";
const OPERATIONS: &str = "result_calendar_correspondence_v1:";

#[derive(Debug, thiserror::Error)]
pub enum ResultCalendarCorrespondenceStoreError {
    #[error("result calendar correspondence requires its protected writer")]
    ReservedWrite,
    #[error("result calendar correspondence requires explicit enrollment")]
    EnrollmentRequired,
    #[error("result calendar correspondence input moved")]
    InputMoved,
    #[error("result calendar correspondence approval differs")]
    ApprovalMismatch,
    #[error("result calendar correspondence operation conflicts")]
    OperationConflict,
    #[error("result calendar correspondence source differs: {0}")]
    SourceMismatch(&'static str),
    #[error("corrupt result calendar correspondence: {0}")]
    Corrupt(&'static str),
    #[error("result calendar correspondence budget exhausted")]
    BudgetExceeded,
    #[error(transparent)]
    Domain(#[from] ResultCalendarCorrespondenceError),
}
use ResultCalendarCorrespondenceStoreError as Error;

pub struct ResultCalendarStartupValidationBudgetV1 {
    grant: AttachmentReplayGrantV1,
}
impl ResultCalendarStartupValidationBudgetV1 {
    pub fn new() -> Self {
        Self {
            grant: AttachmentReplayGrantV1::new(),
        }
    }
}
impl Default for ResultCalendarStartupValidationBudgetV1 {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn refuse_generic_phase(phase: &str) -> StoreResult<()> {
    if phase.starts_with(NAME) {
        return Err(Error::ReservedWrite.into());
    }
    Ok(())
}
pub(crate) fn refuse_generic_operation(operation: &str) -> StoreResult<()> {
    if is_reserved_operation(operation.as_bytes()) {
        return Err(Error::ReservedWrite.into());
    }
    Ok(())
}
pub(crate) fn is_reserved_operation(operation: &[u8]) -> bool {
    operation.starts_with(NAME.as_bytes())
}

fn grant_error(
    error: census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError,
) -> StoreError {
    crate::EvidenceSubjectAttachmentStoreError::Domain(error).into()
}
fn guarded<T>(
    grant: &mut AttachmentReplayGrantV1,
    read: impl FnOnce(&mut AttachmentReplayGrantV1) -> StoreResult<T>,
) -> StoreResult<T> {
    grant.ensure_available().map_err(grant_error)?;
    let result = read(grant);
    if result.is_err() {
        grant.poison();
    }
    result
}
