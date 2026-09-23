//! The store's error taxonomy: what every fallible store call returns, and what each failure names.

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Store failures: fjall, row encoding, journal bounds, counters and sidecar I/O.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// The database or a keyspace could not be opened.
    #[error("store open failed: {source}")]
    Open {
        #[source]
        source: fjall::Error,
    },
    /// The write-ahead journal could not be flushed.
    #[error("store flush failed: {source}")]
    Flush {
        #[source]
        source: fjall::Error,
    },
    /// A read or range scan failed.
    #[error("store read failed: {source}")]
    Read {
        #[source]
        source: fjall::Error,
    },
    /// An append or batch commit failed.
    #[error("store write failed: {source}")]
    Write {
        #[source]
        source: fjall::Error,
    },
    /// A stored row is not valid JSON.
    #[error("row {key} is not valid json: {source}")]
    Decode {
        key: String,
        #[source]
        source: serde_json::Error,
    },
    /// A row did not encode to JSON, or unkeyed bytes did not decode: `detail` names what failed.
    #[error("{detail}: {source}")]
    Json {
        detail: String,
        #[source]
        source: serde_json::Error,
    },
    /// A row of a JSONL snapshot file did not decode: `path` and `line` name the row to repair.
    #[error("snapshot {path} line {line} is not a valid row: {source}")]
    SnapshotRow {
        path: std::path::PathBuf,
        line: usize,
        #[source]
        source: serde_json::Error,
    },
    /// A key, counter or row id violated an invariant the store's writer maintains.
    #[error("{detail}")]
    Invariant { detail: String },
    /// One scan would exceed the configured row ceiling.
    #[error("table {table} would exceed {max} rows in one scan")]
    TooManyRows { table: String, max: usize },
    /// A resume-journal write past the ceiling that bounds its key or its serialized value.
    #[error("journal {what} for {phase}/{key} holds {bytes} bytes, past the {max} ceiling")]
    JournalTooLarge {
        /// Which half of the entry broke the ceiling: `"key"` or `"value"`.
        what: &'static str,
        /// Phase the entry was being written under.
        phase: String,
        /// The caller's key, truncated to what a message can carry.
        key: String,
        /// Bytes the offending half held.
        bytes: usize,
        /// The ceiling it broke.
        max: usize,
    },
    /// The sequence counter at the end of its range.
    #[error("sequence counter overflow")]
    CounterOverflow,
    /// A sidecar or artifact file operation failed.
    #[error("i/o failed for {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// A backup or restore was refused: `detail` names the condition the request did not meet, and what
    /// to do about it. A refusal is never a partial result - the destination is left as it was.
    #[error("{detail}")]
    Refused { detail: String },
    /// The one-time legacy journal import failed.
    #[error("legacy import failed: {detail}")]
    Legacy { detail: String },
}

/// Result alias for store code.
pub type StoreResult<T> = std::result::Result<T, StoreError>;
