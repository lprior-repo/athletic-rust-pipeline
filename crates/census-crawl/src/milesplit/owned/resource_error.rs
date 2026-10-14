use crate::net::FetchOutcome;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnedResourceReceipt {
    NotRequested,
    Retained,
    ReportOnly,
}

#[derive(Debug)]
pub enum OwnedCaptureRetention {
    Unrecorded,
    Captured {
        capture: Box<FetchOutcome>,
        immutable_body: PathBuf,
        receipt: OwnedResourceReceipt,
    },
}

impl fmt::Display for OwnedCaptureRetention {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unrecorded => Ok(()),
            Self::Captured {
                capture,
                immutable_body,
                receipt,
            } => {
                write!(
                    formatter,
                    "; owned capture {} remains unfinished; immutable bytes: {}",
                    capture.content_digest,
                    immutable_body.display()
                )?;
                match receipt {
                    OwnedResourceReceipt::NotRequested | OwnedResourceReceipt::Retained => Ok(()),
                    OwnedResourceReceipt::ReportOnly => formatter.write_str(
                        "; recording cannot retain the partial receipt; immutable capture locator is report-only",
                    ),
                }
            }
        }
    }
}

impl OwnedCaptureRetention {
    pub(crate) fn captured(
        mut capture: FetchOutcome,
        immutable_body: PathBuf,
        receipt: OwnedResourceReceipt,
    ) -> Self {
        capture.body = Vec::new();
        Self::Captured {
            capture: Box::new(capture),
            immutable_body,
            receipt,
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{resource} reservation of {requested} exceeds limit {limit}{retention}")]
pub struct OwnedCaptureResourceFailure {
    resource: &'static str,
    requested: usize,
    limit: usize,
    retention: OwnedCaptureRetention,
}

struct OwnedCaptureResourceOrigin<'a>(&'a OwnedCaptureResourceFailure);

impl fmt::Display for OwnedCaptureResourceOrigin<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let OwnedCaptureResourceFailure {
            resource,
            requested,
            limit,
            retention: _,
        } = self.0;
        write!(
            formatter,
            "{resource} reservation of {requested} exceeds limit {limit}"
        )
    }
}

impl OwnedCaptureResourceFailure {
    pub(crate) fn stable_origin(&self) -> impl fmt::Display + '_ {
        OwnedCaptureResourceOrigin(self)
    }

    pub(crate) fn new(
        resource: &'static str,
        requested: usize,
        limit: usize,
        capture: FetchOutcome,
        immutable_body: PathBuf,
        receipt: OwnedResourceReceipt,
    ) -> Self {
        Self {
            resource,
            requested,
            limit,
            retention: OwnedCaptureRetention::captured(capture, immutable_body, receipt),
        }
    }

    pub fn resource(&self) -> &'static str {
        self.resource
    }

    pub fn requested(&self) -> usize {
        self.requested
    }

    pub fn limit(&self) -> usize {
        self.limit
    }

    pub fn retention(&self) -> &OwnedCaptureRetention {
        &self.retention
    }
}
