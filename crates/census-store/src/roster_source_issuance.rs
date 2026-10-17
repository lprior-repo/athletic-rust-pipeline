use crate::StoreResult;

mod budget;
mod codec;
mod enrollment;
mod format;
mod inspection;
pub use inspection::{RosterIssuanceInspectionRequestV1, RosterIssuanceInspectionV1, RosterSourceEvidenceV1, RosterPhysicalProjectionRowV1};

pub use budget::RosterStartupValidationBudgetV1;
pub use enrollment::{PreparedRosterEnrollmentV1, RosterEnrollmentCommandV1, RosterReviewActorV1,
    RosterEnrollmentInputsV1, RosterEnrollmentReviewV1};
pub(crate) use format::validate_format;

pub(crate) const ROOT: &str = "roster_source_issuance/";
pub(crate) const OPERATION_PREFIX: &str = "roster_source_issuance_v1:";
pub(crate) const ENROLLMENT: &str = "roster_source_issuance/v1/enrollment";
pub(crate) const HEAD: &str = "roster_source_issuance/v1/head";
pub const ROSTER_SOURCE_ISSUANCE_LIFECYCLE: u32 = 8;
pub(crate) const LIFECYCLE: u32 = ROSTER_SOURCE_ISSUANCE_LIFECYCLE;

#[derive(Debug, thiserror::Error)]
pub enum RosterSourceIssuanceStoreError {
    #[error("roster source authority requires its protected writer")]
    ReservedWrite,
    #[error("roster source authority requires explicit lifecycle enrollment")]
    EnrollmentRequired,
    #[error("roster source authority operation conflicts with retained approval")]
    OperationConflict,
    #[error("roster source authority input changed")]
    InputMoved,
    #[error("roster source authority approval differs")]
    ApprovalMismatch,
    #[error("roster source authority validation budget exhausted")]
    BudgetExceeded,
    #[error("corrupt roster source authority: {0}")]
    Corrupt(&'static str),
}

pub(crate) fn refuse_generic_phase(phase: &str) -> StoreResult<()> {
    if phase == "roster_source_issuance"
        || phase.starts_with(ROOT)
        || phase.starts_with("roster_source_issuance\0")
    {
        return Err(RosterSourceIssuanceStoreError::ReservedWrite.into());
    }
    Ok(())
}

pub(crate) fn refuse_generic_operation(operation: &str) -> StoreResult<()> {
    if is_reserved_operation(operation.as_bytes()) {
        return Err(RosterSourceIssuanceStoreError::ReservedWrite.into());
    }
    Ok(())
}

pub(crate) fn is_reserved_operation(operation: &[u8]) -> bool {
    operation.starts_with(OPERATION_PREFIX.as_bytes())
}

#[cfg(test)]
mod tests;

fn source_error(error: census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError) -> crate::StoreError {
    crate::EvidenceSubjectAttachmentStoreError::Domain(error).into()
}
