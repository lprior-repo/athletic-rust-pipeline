use crate::StoreResult;
use census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError;

mod external;
pub(crate) use external::MAX_RETAINED_CAPTURE_ENVELOPE_BYTES;
pub use external::{
    RetainedCaptureLeafKindV1, RetainedCaptureLeafV1, RetainedCaptureRootReadV1,
    RetainedCaptureRootSchemaV1, RetainedCaptureRootV1, RetainedCaptureSlotV1,
};

const PHASE: &str = "evidence_subject";
const OPERATION_PREFIX: &str = "evidence_subject_attachment_v1:";

#[derive(Debug, thiserror::Error)]
pub enum EvidenceSubjectAttachmentStoreError {
    #[error("evidence subject attachment namespace requires its protected writer")]
    ReservedWrite,
    #[error("evidence subject attachment operation names a different request")]
    OperationConflict,
    #[error("evidence subject attachment public identity head moved")]
    HeadMoved,
    #[error("evidence subject attachment inventory moved")]
    InventoryMoved,
    #[error("evidence subject attachment accepted target moved")]
    TargetMoved,
    #[error("evidence subject attachment input revisions or lineage moved")]
    InputMoved,
    #[error("evidence subject attachment external object references are unsupported")]
    ExternalObjectUnsupported,
    #[error("corrupt evidence subject attachment: {0}")]
    Corrupt(&'static str),
    #[error("evidence subject attachment domain: {0}")]
    Domain(
        #[from]
        #[source]
        EvidenceSubjectAttachmentError,
    ),
}

pub(crate) fn refuse_generic_phase(phase: &str) -> StoreResult<()> {
    if phase == PHASE
        || phase
            .strip_prefix(PHASE)
            .is_some_and(|suffix| suffix.starts_with('\0'))
    {
        return Err(EvidenceSubjectAttachmentStoreError::ReservedWrite.into());
    }
    Ok(())
}

pub(crate) fn refuse_generic_operation(operation: &str) -> StoreResult<()> {
    if is_reserved_operation(operation.as_bytes()) {
        return Err(EvidenceSubjectAttachmentStoreError::ReservedWrite.into());
    }
    Ok(())
}

pub(crate) fn is_reserved_operation(operation: &[u8]) -> bool {
    operation.starts_with(OPERATION_PREFIX.as_bytes())
}

#[cfg(test)]
const LOGICAL_PREFIX: &str = "attachment/";

#[cfg(test)]
mod tests;
