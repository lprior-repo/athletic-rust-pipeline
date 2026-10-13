#[derive(Debug, thiserror::Error)]
pub enum CaptureWitnessError {
    #[error("retained capture entry {key} is missing")]
    Missing { key: String },
    #[error("retained capture entry {key} has unsupported {field}: {detail}")]
    Unsupported {
        key: String,
        field: &'static str,
        detail: String,
    },
    #[error("retained capture entry {key} is corrupt: {detail}")]
    Corrupt { key: String, detail: String },
    #[error("retained capture entry {key} is foreign: {detail}")]
    Foreign { key: String, detail: String },
    #[error("retained capture entry {key}: {what} budget refused (quantity {needed}, maximum {max}; read grants must be nonzero)")]
    Budget {
        key: String,
        what: &'static str,
        needed: usize,
        max: usize,
    },
    #[error("retained capture entry {key} could not be read: {source}")]
    Store {
        key: String,
        #[source]
        source: Box<census_store::StoreError>,
    },
    #[error("retained capture entry {key} could not reserve {needed} bytes: {source}")]
    Allocation {
        key: String,
        needed: usize,
        #[source]
        source: std::collections::TryReserveError,
    },
}

pub(super) fn corrupt(key: &str, detail: impl Into<String>) -> CaptureWitnessError {
    CaptureWitnessError::Corrupt {
        key: key.into(),
        detail: detail.into(),
    }
}

pub(super) fn foreign(key: &str, detail: impl Into<String>) -> CaptureWitnessError {
    CaptureWitnessError::Foreign {
        key: key.into(),
        detail: detail.into(),
    }
}
