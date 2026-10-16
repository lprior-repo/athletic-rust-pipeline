#[derive(Debug, thiserror::Error)]
pub enum RetainedMetadataError {
    #[error("raw metadata archive reference is not metadata/{{64 lowercase hex}}/manifest")]
    InvalidArchiveReference,
    #[error("raw metadata archive object is missing: {key}")]
    Missing { key: String },
    #[error("raw metadata archive is invalid: {detail}")]
    Invalid { detail: &'static str },
    #[error("raw metadata and owned result do not name the same meet/result-set")]
    ForeignResult,
    #[error("raw metadata body exceeds the declared read ceiling")]
    BodyBudget,
    #[error("retained result session exceeds sixteen resident raw metadata captures")]
    SessionBudget,
    #[error("raw metadata allocation failed")]
    Allocation(#[from] std::collections::TryReserveError),
    #[error(transparent)]
    Store(#[from] census_store::StoreError),
    #[error(transparent)]
    Encoding(#[from] serde_json::Error),
    #[error(transparent)]
    Source(#[from] crate::CrawlError),
    #[error(transparent)]
    OwnedResult(#[from] crate::milesplit::RetainedResultError),
}

pub(super) fn invalid(detail: &'static str) -> RetainedMetadataError {
    RetainedMetadataError::Invalid { detail }
}
