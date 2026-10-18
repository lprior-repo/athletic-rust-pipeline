use crate::{StoreError, StoreResult};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::result_school_correspondence::*;

mod codec;
mod current;
mod enrollment;
mod issuance;
mod proof;
mod read;
mod source;
mod wire;

pub use current::CurrentResultSchoolCorrespondencesV1;
pub use enrollment::{
    PreparedResultSchoolEnrollmentV1, ResultSchoolEnrollmentInputsV1,
    ResultSchoolEnrollmentReviewV1,
};
pub use issuance::{
    PreparedResultSchoolCorrespondenceV1, ResultSchoolCorrespondenceInspectionV1,
    ResultSchoolCorrespondencePreparationV1,
};
pub use proof::{
    replay_result_school_enrollment_proof_v1, replay_result_school_source_proof_v1,
    ReplayedResultSchoolCorrespondencesV1, ResultSchoolEnrollmentProofV1,
    ResultSchoolSourceProofV1,
};
pub(crate) use read::{preserved_prior_capability, validate_format};

pub const RESULT_SCHOOL_CORRESPONDENCE_LIFECYCLE: u32 = 9;
const NAME: &str = "result_school_correspondence";
const ENROLLMENT: &str = "result_school_correspondence/v1/enrollment";
const OPERATIONS: &str = "result_school_correspondence_v1:";
const LIFECYCLE: u32 = RESULT_SCHOOL_CORRESPONDENCE_LIFECYCLE;

#[derive(Debug, thiserror::Error)]
pub enum ResultSchoolCorrespondenceStoreError {
    #[error("result school correspondence requires its protected writer")]
    ReservedWrite,
    #[error("result school correspondence requires explicit enrollment")]
    EnrollmentRequired,
    #[error("result school correspondence input moved")]
    InputMoved,
    #[error("result school correspondence approval differs")]
    ApprovalMismatch,
    #[error("result school correspondence operation conflicts")]
    OperationConflict,
    #[error("result school correspondence source differs: {0}")]
    SourceMismatch(&'static str),
    #[error("corrupt result school correspondence: {0}")]
    Corrupt(&'static str),
    #[error("result school correspondence budget exhausted")]
    BudgetExceeded,
    #[error(transparent)]
    Domain(#[from] ResultSchoolCorrespondenceError),
}
use ResultSchoolCorrespondenceStoreError as Error;

pub struct ResultSchoolStartupValidationBudgetV1 {
    grant: AttachmentReplayGrantV1,
}
impl ResultSchoolStartupValidationBudgetV1 {
    pub fn new() -> Self {
        Self {
            grant: AttachmentReplayGrantV1::new(),
        }
    }
}
impl Default for ResultSchoolStartupValidationBudgetV1 {
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

#[cfg(test)]
mod enrollment_tests;
#[cfg(test)]
mod issuance_tests;
#[cfg(test)]
mod result_school_guard_tests;
