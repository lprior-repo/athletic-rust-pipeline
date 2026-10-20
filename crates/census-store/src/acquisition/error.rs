use census_domain::model::CaptureDomainError;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PreparedCandidateRejection {
    #[error("candidate length {observed} differs from {expected}")]
    WrongLength { expected: u64, observed: u64 },
    #[error("candidate digest differs from the fixed Prepared digest")]
    WrongDigest,
    #[error("candidate is not a regular file")]
    NonRegular,
    #[error("candidate ancestry is unsafe")]
    UnsafeParent,
    #[error("candidate changed while being read")]
    Changed,
    #[error("candidate length {observed} exceeds {limit}")]
    TooLarge { limit: u64, observed: u64 },
    #[error("candidate path is unsupported")]
    UnsupportedPath,
}

#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    #[error(transparent)]
    Domain(#[from] CaptureDomainError),
    #[error("capture qualification requires explicit opt-in")]
    CaptureQualificationOptInRequired,
    #[error("unsupported capture protocol")]
    CaptureProtocolUnsupported,
    #[error("capture lifecycle marker does not match its records")]
    CaptureMarkerMismatch,
    #[error("capture writer capability is missing")]
    CaptureCapabilityMissing,
    #[error("capture capability belongs to another Store handle or lineage")]
    CaptureCapabilityForeignStore,
    #[error("unsupported capture capability namespace")]
    CaptureCapabilityNamespaceUnsupported,
    #[error("capture init requires a genuinely empty root")]
    CaptureInitRootNotEmpty,
    #[error("capture init refuses an existing store")]
    CaptureInitExistingStore,
    #[error("capture requires a bound run manifest")]
    CaptureRunManifestMissing,
    #[error("capture run or retained manifest does not match")]
    CaptureRunMismatch,
    #[error("capture jurisdiction is not in its run manifest")]
    CaptureJurisdictionOutsideManifest,
    #[error("capture owner key does not match")]
    CaptureOwnerKeyMismatch,
    #[error("capture history does not match")]
    CaptureHistoryConflict,
    #[error("capture role is unsupported")]
    CaptureRoleUnsupported,
    #[error("this acquisition slot is bound to different immutable work")]
    ObligationConflict,
    #[error("acquisition slot/work binding is missing or mismatched")]
    SlotWorkMismatch,
    #[error("acquisition work digest does not match its identity")]
    WorkDigestMismatch,
    #[error("acquisition lifetime budget is exhausted")]
    AcquisitionBudgetExhausted,
    #[error("retained acquisition includes an unknown physical outcome")]
    RetainedOutcomeUnknown,
    #[error("historical capture has no exact attempt provenance")]
    HistoricalAttemptUnknown,
    #[error("unsupported eligible cache envelope")]
    CacheEnvelopeUnsupported,
    #[error("cache occurrence does not match")]
    CacheOccurrenceMismatch,
    #[error("required acquisition backing is missing")]
    RequiredBackingMissing,
    #[error("acquisition backing bytes do not match their retained addresses")]
    BackingIntegrity,
    #[error("capture cache root is not this Store's http root")]
    CacheRootMismatch,
    #[error("acquisition state does not permit this transition")]
    TransitionRefused,
    #[error("the attempt terminal is immutable")]
    AttemptTerminalConflict,
    #[error("capture recovery is required before dispatch")]
    RecoveryRequired,
    #[error("capture prepared recovery must finish the original attempt")]
    PreparedRecoveryRequired,
    #[error("exact prepared backing is available for retention")]
    ExactRecoveryAvailable,
    #[error("prepared backing changed after its assessment")]
    PreparedAssessmentChanged,
    #[error("occupied prepared backing requires authorized repair")]
    PreparedRepairRequired,
    #[error("prepared recovery target does not match its immutable facts")]
    PreparedTargetMismatch,
    #[error("prepared body candidate was rejected: {reason}")]
    PreparedCandidateRejected { reason: PreparedCandidateRejection },
    #[error("acquisition read exceeds its shared {resource} budget")]
    ReadBudgetExceeded { resource: &'static str },
    #[error("acquisition read for {resource} requires {required}, exceeding its limit of {limit}")]
    ReadBudgetLimitExceeded {
        resource: &'static str,
        required: u64,
        limit: u64,
    },
    #[error("allocation reservation failed for {resource} (requested {requested_bytes:?} bytes): {source}")]
    AllocationReservationFailed {
        resource: &'static str,
        requested_bytes: Option<usize>,
        #[source]
        source: std::collections::TryReserveError,
    },
    #[error("allocation accounting invariant failed for {resource}")]
    AllocationAccountingInvariant { resource: &'static str },
    #[error("unsupported or surplus acquisition root")]
    UnsupportedRoot,
    #[error("malformed acquisition closure")]
    CorruptClosure,
    #[error("capture record exceeds its inline byte bound")]
    RecordTooLarge,
}

impl CaptureError {
    pub(crate) fn allocation_reservation_failed(
        resource: &'static str,
        requested_bytes: Option<usize>,
        source: std::collections::TryReserveError,
    ) -> Self {
        Self::AllocationReservationFailed {
            resource,
            requested_bytes,
            source,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CaptureError;

    #[test]
    fn legacy_read_budget_variant_retains_its_shape_and_display() {
        let error = CaptureError::ReadBudgetExceeded { resource: "roots" };
        assert_eq!(
            error.to_string(),
            "acquisition read exceeds its shared roots budget"
        );
        assert!(matches!(
            error,
            CaptureError::ReadBudgetExceeded { resource: "roots" }
        ));
    }
}
