use super::{CaptureDomainError, CAPTURE_QUALIFICATION_KEY, SOURCE_ACQUISITION_PHASE};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub enum CaptureProtocolVersion {
    V1,
    V2,
}
impl CaptureProtocolVersion {
    pub fn phase(self) -> &'static str {
        match self {
            Self::V1 => SOURCE_ACQUISITION_PHASE,
            Self::V2 => "source_acquisition_attempt_v2",
        }
    }
    pub fn qualification_key(self) -> &'static str {
        match self {
            Self::V1 => CAPTURE_QUALIFICATION_KEY,
            Self::V2 => "capture:v2:qualification",
        }
    }
    pub fn wire_version(self) -> u8 {
        self.into()
    }
    pub fn permits_prepared_missing_close(self) -> bool {
        matches!(self, Self::V2)
    }
}
impl TryFrom<u8> for CaptureProtocolVersion {
    type Error = CaptureDomainError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::V1),
            2 => Ok(Self::V2),
            _ => Err(CaptureDomainError::Version(value)),
        }
    }
}
impl From<CaptureProtocolVersion> for u8 {
    fn from(value: CaptureProtocolVersion) -> Self {
        match value {
            CaptureProtocolVersion::V1 => 1,
            CaptureProtocolVersion::V2 => 2,
        }
    }
}
