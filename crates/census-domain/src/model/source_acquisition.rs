mod error;
mod identity;
mod protocol;
mod qualification;
mod request;
mod scalar;
mod work;

pub use error::CaptureDomainError;
pub use identity::{
    AcquisitionAttemptId, AcquisitionDispatchId, AcquisitionOccurrenceRefV1, AcquisitionSlotV1,
};
pub use protocol::{
    AcquisitionFailureV1, CaptureDispositionV1, InterruptionCauseV1, PreparedCaptureInputV1,
    ResponseCheckpointRefusalV1, ResponseObservationV1, ResponseStatusV1, ResponseUrlV1,
};
pub use qualification::CaptureQualificationV1;
pub use request::{ConditionalRequestV1, DispatchRequestV1, RequestRepresentationV1};
pub use scalar::{
    AcquisitionDigest, AcquisitionSlotId, AcquisitionWorkId, AttemptOrdinal, HopOrdinal,
};
pub use work::{
    AcquisitionWorkV1, CaptureCacheIntent, ResultsCaptureScopeV1, SelectedHistoryV1,
    SourceAdmissionV1,
};

pub const SOURCE_ACQUISITION_PHASE: &str = "source_acquisition_attempt_v1";
pub const CAPTURE_QUALIFICATION_KEY: &str = "capture:v1:qualification";
pub const CAPTURE_QUALIFICATION_LIFECYCLE: u32 = 4;
pub const MAX_ACQUISITION_ROOTS: usize = 42;
pub const MAX_CAPTURE_RECORD_BYTES: usize = 65_536;
pub const MAX_ACQUISITION_BODY_BYTES: u64 = 32 * 1024 * 1024;

#[cfg(test)]
mod tests;
