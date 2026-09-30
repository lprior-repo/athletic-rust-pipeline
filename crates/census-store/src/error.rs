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
    #[error(
        "operation {operation} was staged against evidence generation {expected}, but the store is          at {actual}"
    )]
    EvidenceMoved {
        operation: String,
        expected: u64,
        actual: u64,
    },
    #[error("identity projection failed: {0}")]
    Identity(#[from] crate::identity::IdentityError),
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
    #[error("table {table} is derived; replace its rows by id instead of appending")]
    DerivedAppend { table: &'static str },
    #[error("i/o failed for {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{detail}")]
    Refused { detail: String },
    #[error(
        "store {what} is version {found}, newer than this build's {supported}: refusing to open"
    )]
    FormatNewer {
        what: &'static str,
        found: u32,
        supported: u32,
    },
    #[error("store at {root} needs an explicit migration before opening: {detail}")]
    MigrationRequired { root: String, detail: String },
    #[error("store schema metadata is unreadable: {detail}")]
    SchemaUnknown { detail: String },
    #[error("table {table} is not stored as derived generations")]
    NotGenerationTable { table: &'static str },
    #[error("derived publication refused: {detail}")]
    PublicationRefused { detail: String },
}

pub type StoreResult<T> = std::result::Result<T, StoreError>;
