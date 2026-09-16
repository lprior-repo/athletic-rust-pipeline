use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("artifact document exceeds the configured size limit")]
    DocumentTooLarge,
    #[error("source publication exceeds the configured batch limit")]
    BatchTooLarge,
    #[error("source index page limit must be in 1..=1024")]
    InvalidPageLimit,
    #[error("source row key is inconsistent with its stored row")]
    InvalidSourceRow,
    #[error("source row conflicts with an existing publication")]
    SourceConflict,
    #[error("requested artifact is absent")]
    MissingArtifact,
    #[error("artifact digest verification failed")]
    DigestMismatch,
    #[error("artifact store root is not a private directory")]
    InsecureRoot,
    #[error("stored artifact or source index is corrupt")]
    CorruptData,
    #[error("artifact database I/O failed")]
    DatabaseIo,
    #[error("artifact database operation failed")]
    Database,
    #[error("source record serialization failed")]
    Serialization,
    #[error("source visitor failed")]
    Visitor,
}

pub type Result<T, E = StoreError> = std::result::Result<T, E>;

impl From<serde_json::Error> for StoreError {
    fn from(_: serde_json::Error) -> Self {
        Self::Serialization
    }
}

pub fn map_database_error(error: fjall::Error) -> StoreError {
    match error {
        fjall::Error::Io(_) => StoreError::DatabaseIo,
        fjall::Error::Storage(_)
        | fjall::Error::JournalRecovery(_)
        | fjall::Error::InvalidVersion(_)
        | fjall::Error::Decompress(_)
        | fjall::Error::InvalidTrailer
        | fjall::Error::InvalidTag(_)
        | fjall::Error::Unrecoverable => StoreError::CorruptData,
        fjall::Error::Poisoned | fjall::Error::KeyspaceDeleted | fjall::Error::Locked => {
            StoreError::Database
        }
        _ => StoreError::Database,
    }
}
