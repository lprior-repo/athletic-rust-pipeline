use http::{HeaderMap, Method, StatusCode, Uri};

use super::{AdmissionKind, IngressError, ResponseRefusal};

const DATA_BYTES: usize = 64 * 1024 * 1024;
const CONTROL_BYTES: usize = 64 * 1024;
const FAILURE_BYTES: usize = 256 * 1024;
const HEADER_BYTES: usize = 64 * 1024;
const HEADER_COUNT: usize = 256;
const RETAINED_INVOCATION_BYTES: usize = 128;
const EMPTY_CHUNKS: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseClass {
    Data,
    Control,
    Failure,
}

impl ResponseClass {
    const fn limit(self) -> usize {
        match self {
            Self::Data => DATA_BYTES,
            Self::Control => CONTROL_BYTES,
            Self::Failure => FAILURE_BYTES,
        }
    }
}

pub(super) fn is_control(method: &Method, uri: &Uri) -> bool {
    method == Method::POST
        && (uri.path().ends_with("/send") || uri.path() == "/Census/publication_capabilities")
}

pub(super) fn requested_invocation(uri: &Uri) -> Option<String> {
    let path = uri.path().strip_prefix("/restate/invocation/")?;
    let (id, operation) = path.rsplit_once('/')?;
    (!id.is_empty()
        && id.len() <= RETAINED_INVOCATION_BYTES
        && !id.contains('/')
        && matches!(operation, "output" | "attach"))
    .then(|| id.to_owned())
}

pub(super) struct Admission {
    class: ResponseClass,
    status: u16,
    invocation_id: Option<String>,
    pub(super) limit: usize,
    pub(super) empty_chunks: usize,
}

impl Admission {
    pub(super) fn new(control: bool, status: StatusCode, headers: &HeaderMap) -> Self {
        let class = if !status.is_success() {
            ResponseClass::Failure
        } else if control {
            ResponseClass::Control
        } else {
            ResponseClass::Data
        };
        let invocation_id = headers
            .get("x-restate-id")
            .filter(|value| value.len() <= RETAINED_INVOCATION_BYTES)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        Self {
            class,
            status: status.as_u16(),
            invocation_id,
            limit: class.limit(),
            empty_chunks: EMPTY_CHUNKS,
        }
    }

    pub(super) fn with_requested_invocation(mut self, requested: Option<String>) -> Self {
        if requested.is_some() {
            self.invocation_id = requested;
        }
        self
    }

    pub(super) fn transport_error(&self, source: reqwest::Error) -> IngressError {
        super::error::response_error(source, self.invocation_id.as_deref(), Some(self.status))
    }

    pub(super) fn refusal(
        &self,
        kind: AdmissionKind,
        limit: usize,
        observed: usize,
        declared: Option<u64>,
    ) -> ResponseRefusal {
        ResponseRefusal {
            kind,
            class: self.class,
            status: self.status,
            limit,
            observed,
            declared,
            invocation_id: self.invocation_id.clone(),
        }
    }

    pub(super) fn headers(&self, headers: &HeaderMap) -> Result<(), IngressError> {
        if headers.len() > HEADER_COUNT {
            return Err(self
                .refusal(
                    AdmissionKind::HeaderCount,
                    HEADER_COUNT,
                    headers.len(),
                    None,
                )
                .into());
        }
        headers.iter().try_fold(0usize, |bytes, (name, value)| {
            let next = bytes
                .checked_add(name.as_str().len())
                .and_then(|bytes| bytes.checked_add(value.len()))
                .ok_or_else(|| {
                    self.refusal(AdmissionKind::AccountingOverflow, HEADER_BYTES, bytes, None)
                })?;
            if next > HEADER_BYTES {
                return Err(self
                    .refusal(AdmissionKind::HeaderBytes, HEADER_BYTES, next, None)
                    .into());
            }
            Ok::<_, IngressError>(next)
        })?;
        Ok(())
    }

    pub(super) fn declared(&self, length: Option<u64>) -> Result<(), IngressError> {
        let limit = u64::try_from(self.limit)
            .map_err(|_| self.refusal(AdmissionKind::AccountingOverflow, self.limit, 0, length))?;
        if length.is_some_and(|length| length > limit) {
            return Err(self
                .refusal(AdmissionKind::DeclaredBodyBytes, self.limit, 0, length)
                .into());
        }
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn with_empty_limit(mut self, limit: usize) -> Self {
        self.empty_chunks = limit;
        self
    }

    #[cfg(test)]
    pub(super) fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }
}

#[cfg(test)]
mod tests;
