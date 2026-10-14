pub use super::super::resource_error::{
    OwnedCaptureResourceFailure, OwnedCaptureRetention, OwnedResourceReceipt,
};
use crate::net::FetchOutcome;
use std::collections::TryReserveError;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum OwnedMeetParseError {
    #[error(transparent)]
    Malformed(#[from] OwnedDocumentFailure),
    #[error(transparent)]
    Resource(#[from] OwnedParseResourceFailure),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnedParseLocator {
    RequestedMeet,
    Document,
    Data,
    ProviderStatus,
    EmbeddedMeet,
    Byte { offset: usize },
}

impl fmt::Display for OwnedParseLocator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RequestedMeet => formatter.write_str("requested meet ID"),
            Self::Document => formatter.write_str("document"),
            Self::Data => formatter.write_str("data"),
            Self::ProviderStatus => formatter.write_str("/_meta/status_code"),
            Self::EmbeddedMeet => formatter.write_str("/_embedded/meet/id"),
            Self::Byte { offset } => write!(formatter, "byte[{offset}]"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OwnedDocumentFailure {
    #[error("invalid meet ID or oversized structured response")]
    InvalidRequestedMeet { locator: OwnedParseLocator },
    #[error("{source}")]
    Json {
        locator: OwnedParseLocator,
        #[source]
        source: serde_json::Error,
    },
    #[error("provider envelope status is {published}")]
    ProviderStatus {
        locator: OwnedParseLocator,
        published: String,
    },
    #[error("embedded meet ID {published} differs from requested meet {requested}")]
    EmbeddedMeet {
        locator: OwnedParseLocator,
        published: String,
        requested: u64,
    },
    #[error("expected a JSON object with a data array")]
    DataArray { locator: OwnedParseLocator },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnedParseResource {
    CaptureBytes,
    ArrayValues,
    JsonDepth,
    DecodeWork,
    DecodedAllocation,
    ResultRows,
    AcceptedRows,
    RejectedRows,
    ResultIdentities,
    EnvelopeDiagnostic,
}

impl fmt::Display for OwnedParseResource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::CaptureBytes => "owned capture bytes",
            Self::ArrayValues => "owned array values",
            Self::JsonDepth => "owned JSON depth",
            Self::DecodeWork => "owned decode work",
            Self::DecodedAllocation => "owned decoded allocation",
            Self::ResultRows => "owned structured rows",
            Self::AcceptedRows => "owned accepted rows",
            Self::RejectedRows => "owned rejected rows",
            Self::ResultIdentities => "owned result identities",
            Self::EnvelopeDiagnostic => "owned envelope diagnostic",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnedParseArithmetic {
    TokenIncrement,
    ArrayIncrement,
    DepthIncrement,
    DepthFrame,
    TextBytes,
    NodeBytes,
    RowBytes,
    VectorBytes,
    TotalBytes,
    DiagnosticBytes,
}

#[derive(Debug, thiserror::Error)]
pub enum OwnedParseResourceFailure {
    #[error("{resource} reservation of {requested} exceeds limit {limit} at {locator}{retention}")]
    Limit {
        resource: OwnedParseResource,
        locator: OwnedParseLocator,
        requested: usize,
        limit: usize,
        retention: OwnedCaptureRetention,
    },
    #[error("arithmetic overflow: owned decode admission overflow ({resource}/{operation:?}) at {locator}{retention}")]
    Arithmetic {
        resource: OwnedParseResource,
        locator: OwnedParseLocator,
        operation: OwnedParseArithmetic,
        retention: OwnedCaptureRetention,
    },
    #[error("{resource} reservation of {requested} failed: {source} at {locator}{retention}")]
    Allocation {
        resource: OwnedParseResource,
        locator: OwnedParseLocator,
        requested: usize,
        #[source]
        source: TryReserveError,
        retention: OwnedCaptureRetention,
    },
}

struct OwnedParseResourceOrigin<'a>(&'a OwnedParseResourceFailure);

impl fmt::Display for OwnedParseResourceOrigin<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            OwnedParseResourceFailure::Limit {
                resource,
                locator,
                requested,
                limit,
                retention: _,
            } => write!(
                formatter,
                "{resource} reservation of {requested} exceeds limit {limit} at {locator}"
            ),
            OwnedParseResourceFailure::Arithmetic {
                resource,
                locator,
                operation,
                retention: _,
            } => write!(
                formatter,
                "arithmetic overflow: owned decode admission overflow ({resource}/{operation:?}) at {locator}"
            ),
            OwnedParseResourceFailure::Allocation {
                resource,
                locator,
                requested,
                source,
                retention: _,
            } => write!(
                formatter,
                "{resource} reservation of {requested} failed: {source} at {locator}"
            ),
        }
    }
}

impl OwnedParseResourceFailure {
    pub(crate) fn stable_origin(&self) -> impl fmt::Display + '_ {
        OwnedParseResourceOrigin(self)
    }

    pub fn locator(&self) -> OwnedParseLocator {
        match self {
            Self::Limit { locator, .. }
            | Self::Arithmetic { locator, .. }
            | Self::Allocation { locator, .. } => *locator,
        }
    }

    pub fn retention(&self) -> &OwnedCaptureRetention {
        match self {
            Self::Limit { retention, .. }
            | Self::Arithmetic { retention, .. }
            | Self::Allocation { retention, .. } => retention,
        }
    }

    pub(crate) fn retain(
        mut self,
        capture: FetchOutcome,
        immutable_body: PathBuf,
        receipt: OwnedResourceReceipt,
    ) -> Self {
        let retained = OwnedCaptureRetention::captured(capture, immutable_body, receipt);
        match &mut self {
            Self::Limit { retention, .. }
            | Self::Arithmetic { retention, .. }
            | Self::Allocation { retention, .. } => *retention = retained,
        }
        self
    }

    pub(super) fn limit(
        resource: OwnedParseResource,
        locator: OwnedParseLocator,
        requested: usize,
        limit: usize,
    ) -> Result<(), Self> {
        if requested > limit {
            return Err(Self::Limit {
                resource,
                locator,
                requested,
                limit,
                retention: OwnedCaptureRetention::Unrecorded,
            });
        }
        Ok(())
    }

    pub(super) fn arithmetic(
        resource: OwnedParseResource,
        locator: OwnedParseLocator,
        operation: OwnedParseArithmetic,
    ) -> Self {
        Self::Arithmetic {
            resource,
            locator,
            operation,
            retention: OwnedCaptureRetention::Unrecorded,
        }
    }

    pub(super) fn allocation(
        resource: OwnedParseResource,
        locator: OwnedParseLocator,
        requested: usize,
        source: TryReserveError,
    ) -> Self {
        Self::Allocation {
            resource,
            locator,
            requested,
            source,
            retention: OwnedCaptureRetention::Unrecorded,
        }
    }
}
