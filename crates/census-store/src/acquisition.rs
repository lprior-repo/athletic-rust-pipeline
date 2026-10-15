pub(crate) mod admission;
mod backing;
mod budget;
pub(crate) mod cache;
mod capability;
pub(crate) mod codec;
mod error;
mod facts;
pub(crate) mod init;
pub(crate) mod qualification;
mod read;
mod recovery;
mod terminal;
mod transition;

pub use budget::{AcquisitionReadBudget, AcquisitionSelectionUsage};
pub use capability::{
    ArmedDispatch, CaptureWriterV1, DispatchObservation, ObservedResponse, OwnedAcquisitionWork,
    PreparedCapture, ReservedAttempt, ResultsCaptureOwnerV1, RetryOwnerGrant,
};
pub use error::{CaptureError, PreparedCandidateRejection};
pub use facts::{
    AcquireDecision, AcquisitionCheckpointV1, AcquisitionTerminal, AcquisitionTerminalV1,
    ImmutableAttemptResult,
};
pub(crate) use qualification::{require_legacy, validate_format, validate_preservation};
pub use qualification::{CaptureQualificationControl, CaptureQualificationStore};
pub(crate) use read::verify_reference;
pub use read::{ExistingRecoveryRoute, VerifiedRetainedOccurrence};
pub use recovery::{
    ArchiveWriteFence, CommittedRetainedBody, ExactBodySource, PreparedBackingAssessment,
    PreparedRecoveryBudget, PreparedRecoveryRead, PreparedRecoverySession,
    RecordedRetentionFailure, RecoveryArchiveIo, RecoveryFsOperation, RecoveryPayloadRole,
    RetainedCaptureParts,
};

#[cfg(test)]
mod tests;
