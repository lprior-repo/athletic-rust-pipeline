#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("store open failed: {source}")]
    Open {
        #[source]
        source: fjall::Error,
    },
    #[error("store flush failed: {source}")]
    Flush {
        #[source]
        source: fjall::Error,
    },
    #[error("store read failed: {source}")]
    Read {
        #[source]
        source: fjall::Error,
    },
    #[error("store write failed: {source}")]
    Write {
        #[source]
        source: fjall::Error,
    },
    #[error("row {key} is not valid json: {source}")]
    Decode {
        key: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("{detail}: {source}")]
    Json {
        detail: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("snapshot {path} line {line} is not a valid row: {source}")]
    SnapshotRow {
        path: std::path::PathBuf,
        line: usize,
        #[source]
        source: serde_json::Error,
    },
    #[error("{detail}")]
    Invariant { detail: String },
    #[error("identity projection failed: {0}")]
    Identity(#[from] census_domain::model::IdentityError),
    #[error("table {table} would exceed {max} rows in one scan")]
    TooManyRows { table: String, max: usize },
    #[error("journal {what} for {phase}/{key} holds {bytes} bytes, past the {max} ceiling")]
    JournalTooLarge {
        what: &'static str,
        phase: String,
        key: String,
        bytes: usize,
        max: usize,
    },
    #[error("sequence counter overflow")]
    CounterOverflow,
    #[error("observation log {table} is append-only; replacement is refused")]
    ObservationReplacement { table: &'static str },
    #[error("i/o failed for {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{detail}")]
    Refused { detail: String },
}

pub type StoreResult<T> = std::result::Result<T, StoreError>;
