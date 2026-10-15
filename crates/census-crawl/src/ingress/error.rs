use restate_sdk::ingress::ClientError;

use super::ResponseClass;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionKind {
    DeclaredBodyBytes,
    StreamedBodyBytes,
    EmptyChunks,
    HeaderBytes,
    HeaderCount,
    Allocation,
    AccountingOverflow,
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("ingress response admission refused: {kind:?}, class={class:?}, status={status}, limit={limit}, observed={observed}, declared={declared:?}, invocation={invocation_id:?}")]
pub struct ResponseRefusal {
    pub kind: AdmissionKind,
    pub class: ResponseClass,
    pub status: u16,
    pub limit: usize,
    pub observed: usize,
    pub declared: Option<u64>,
    pub invocation_id: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum IngressError {
    #[error(transparent)]
    Transport(#[from] reqwest::Error),
    #[error("ingress response transport failed: status={status:?}, invocation={invocation_id:?}: {source}")]
    ResponseTransport {
        status: Option<u16>,
        invocation_id: Option<String>,
        #[source]
        source: reqwest::Error,
    },
    #[error("ingress HTTP parser size admission refused: status={status:?}, observed={observed:?}, invocation={invocation_id:?}: {source}")]
    ParserSize {
        status: Option<u16>,
        observed: Option<usize>,
        invocation_id: Option<String>,
        #[source]
        source: reqwest::Error,
    },
    #[error(transparent)]
    Refused(#[from] ResponseRefusal),
    #[error("{refusal}: {source}")]
    Allocation {
        refusal: ResponseRefusal,
        #[source]
        source: std::collections::TryReserveError,
    },
}

pub fn is_admission_error(error: &ClientError) -> bool {
    match error {
        ClientError::Transport { source } => {
            source.downcast_ref::<IngressError>().is_some_and(|error| {
                matches!(
                    error,
                    IngressError::Refused(_)
                        | IngressError::Allocation { .. }
                        | IngressError::ParserSize { .. }
                )
            })
        }
        _ => false,
    }
}

pub(super) fn response_error(
    source: reqwest::Error,
    invocation_id: Option<&str>,
    status: Option<u16>,
) -> IngressError {
    let root: &(dyn std::error::Error + 'static) = &source;
    let oversized = std::iter::successors(Some(root), |error| error.source())
        .take(16)
        .any(|error| {
            error
                .downcast_ref::<hyper::Error>()
                .is_some_and(hyper::Error::is_parse_too_large)
        });
    if oversized {
        IngressError::ParserSize {
            status,
            observed: None,
            invocation_id: invocation_id.map(str::to_owned),
            source,
        }
    } else {
        IngressError::ResponseTransport {
            status,
            invocation_id: invocation_id.map(str::to_owned),
            source,
        }
    }
}
