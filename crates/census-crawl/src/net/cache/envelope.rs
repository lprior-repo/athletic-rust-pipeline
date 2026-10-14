use super::{CacheMeta, Capture, FetchError, MAX_META_BYTES};
use crate::net::CaptureTransportError;
use census_domain::model::AcquisitionOccurrenceRefV1;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EligibleCacheV2 {
    version: VersionTwo,
    pub(crate) metadata: CacheMeta,
    pub(crate) original_occurrence: AcquisitionOccurrenceRefV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
struct VersionTwo;

impl TryFrom<u8> for VersionTwo {
    type Error = CaptureTransportError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            2 => Ok(Self),
            _ => Err(CaptureTransportError::CacheEnvelopeUnsupported),
        }
    }
}

impl From<VersionTwo> for u8 {
    fn from(_: VersionTwo) -> Self {
        2
    }
}

pub(super) enum CacheDocument {
    Historical(CacheMeta),
    Eligible(EligibleCacheV2),
}

impl CacheDocument {
    pub(super) fn metadata(self) -> CacheMeta {
        match self {
            Self::Historical(meta) => meta,
            Self::Eligible(envelope) => envelope.metadata,
        }
    }
}

pub(super) fn decode(bytes: &[u8], path: &Path) -> Result<CacheDocument, FetchError> {
    if bytes.len() > MAX_META_BYTES {
        return Err(CaptureTransportError::CacheEnvelopeUnsupported.into());
    }
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|source| FetchError::Decode {
            target: path.display().to_string(),
            source,
        })?;
    if value.get("version").is_some()
        || value.get("original_occurrence").is_some()
        || value.get("metadata").is_some()
    {
        let envelope = serde_json::from_slice(bytes)
            .map_err(|_| CaptureTransportError::CacheEnvelopeUnsupported)?;
        Ok(CacheDocument::Eligible(envelope))
    } else {
        let meta = serde_json::from_slice(bytes).map_err(|source| FetchError::Decode {
            target: path.display().to_string(),
            source,
        })?;
        Ok(CacheDocument::Historical(meta))
    }
}

impl EligibleCacheV2 {
    pub(crate) fn new(
        metadata: CacheMeta,
        original_occurrence: AcquisitionOccurrenceRefV1,
    ) -> Self {
        Self {
            version: VersionTwo,
            metadata,
            original_occurrence,
        }
    }

    pub(crate) fn encode(&self, path: &Path) -> Result<Vec<u8>, FetchError> {
        Capture::from_meta(&self.metadata, path)?;
        let encoded = serde_json::to_vec_pretty(self).map_err(|source| FetchError::Encode {
            target: path.display().to_string(),
            source,
        })?;
        if encoded.len() > MAX_META_BYTES {
            return Err(CaptureTransportError::CacheEnvelopeUnsupported.into());
        }
        Ok(encoded)
    }
}
