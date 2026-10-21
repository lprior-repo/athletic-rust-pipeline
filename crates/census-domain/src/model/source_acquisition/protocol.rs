use super::scalar::url;
use super::{
    AcquisitionDigest, CaptureDomainError, MAX_ACQUISITION_BODY_BYTES, MAX_CAPTURE_RECORD_BYTES,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

mod response;
pub use response::ResponseObservationV1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "PreparedWire")]
pub struct PreparedCaptureInputV1 {
    content_digest: AcquisitionDigest,
    content_bytes: u64,
    metadata_digest: AcquisitionDigest,
    metadata_bytes: Vec<u8>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PreparedWire {
    content_digest: AcquisitionDigest,
    content_bytes: u64,
    metadata_digest: AcquisitionDigest,
    metadata_bytes: Vec<u8>,
}
impl PreparedCaptureInputV1 {
    pub fn new(
        content_digest: &str,
        content_bytes: u64,
        metadata_bytes: Vec<u8>,
    ) -> Result<Self, CaptureDomainError> {
        if content_bytes > MAX_ACQUISITION_BODY_BYTES {
            return Err(CaptureDomainError::Bound {
                field: "body",
                max: 32 * 1024 * 1024,
            });
        }
        if metadata_bytes.is_empty() || metadata_bytes.len() > MAX_CAPTURE_RECORD_BYTES {
            return Err(CaptureDomainError::Bound {
                field: "metadata",
                max: MAX_CAPTURE_RECORD_BYTES,
            });
        }
        let metadata_digest =
            AcquisitionDigest::parse(&format!("{:x}", Sha256::digest(&metadata_bytes)))?;
        Ok(Self {
            content_digest: AcquisitionDigest::parse(content_digest)?,
            content_bytes,
            metadata_digest,
            metadata_bytes,
        })
    }
    pub fn content_digest(&self) -> &AcquisitionDigest {
        &self.content_digest
    }
    pub fn content_bytes(&self) -> u64 {
        self.content_bytes
    }
    pub fn metadata_digest(&self) -> &AcquisitionDigest {
        &self.metadata_digest
    }
    pub fn metadata_bytes(&self) -> &[u8] {
        &self.metadata_bytes
    }
    pub fn into_metadata_bytes(self) -> Vec<u8> {
        self.metadata_bytes
    }
}
impl TryFrom<PreparedWire> for PreparedCaptureInputV1 {
    type Error = CaptureDomainError;
    fn try_from(w: PreparedWire) -> Result<Self, Self::Error> {
        let result = Self::new(w.content_digest.as_str(), w.content_bytes, w.metadata_bytes)?;
        if result.metadata_digest != w.metadata_digest {
            return Err(CaptureDomainError::Invalid {
                field: "metadata digest",
            });
        }
        Ok(result)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum AcquisitionFailureV1 {
    ResponseCheckpointRefused {
        status: ResponseStatusV1,
        response_url: ResponseUrlV1,
        reason: ResponseCheckpointRefusalV1,
    },
    AdmissionRefused,
    SourceStopped,
    Transport,
    Timeout,
    BodyIncomplete,
    BodyTooLarge,
    UnrepresentableResponse,
    ArchiveMissing,
    ArchiveIntegrity,
    CacheChanged,
    RedirectRefused,
    RedirectLimit,
    ResourceLimit,
    UnsupportedResponse,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterruptionCauseV1 {
    OwnerRestarted,
    WorkerCancelled,
    WorkerPanicked,
    WorkerDrained,
    TransportFailed,
    TransportTimedOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureDispositionV1 {
    ReusableResponse,
    RetryableResponse,
    TerminalResponse,
}
impl CaptureDispositionV1 {
    pub fn for_http_status(status: u16) -> Result<Self, CaptureDomainError> {
        if !(100..=599).contains(&status) || status == 304 {
            return Err(CaptureDomainError::Invalid {
                field: "retained response status",
            });
        }
        Ok(match status {
            200 => Self::ReusableResponse,
            429 | 500..=599 => Self::RetryableResponse,
            _ => Self::TerminalResponse,
        })
    }
}

impl AcquisitionFailureV1 {
    pub fn allows_continuation(&self) -> bool {
        matches!(self, Self::Transport | Self::Timeout | Self::BodyIncomplete)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseCheckpointRefusalV1 {
    UnrepresentableResponse,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub struct ResponseStatusV1(u16);
impl ResponseStatusV1 {
    pub fn new(status: u16) -> Result<Self, CaptureDomainError> {
        if !(100..=599).contains(&status) {
            return Err(CaptureDomainError::Invalid {
                field: "response status",
            });
        }
        Ok(Self(status))
    }
    pub fn get(self) -> u16 {
        self.0
    }
}
impl TryFrom<u16> for ResponseStatusV1 {
    type Error = CaptureDomainError;
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<ResponseStatusV1> for u16 {
    fn from(value: ResponseStatusV1) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ResponseUrlV1(String);
impl ResponseUrlV1 {
    pub fn new(value: &str) -> Result<Self, CaptureDomainError> {
        url(value)?;
        Ok(Self(value.to_string()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn is_for_dispatch(&self, request: &super::DispatchRequestV1) -> bool {
        match (url(&self.0), url(request.target_url())) {
            (Ok(actual), Ok(expected)) => actual == expected,
            _ => false,
        }
    }
}
impl TryFrom<String> for ResponseUrlV1 {
    type Error = CaptureDomainError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(&value)
    }
}
impl From<ResponseUrlV1> for String {
    fn from(value: ResponseUrlV1) -> Self {
        value.0
    }
}
