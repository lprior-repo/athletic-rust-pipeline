#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CaptureDomainError {
    #[error("unsupported capture version {0}")]
    Version(u8),
    #[error("invalid capture {field}")]
    Invalid { field: &'static str },
    #[error("capture {field} exceeds {max} bytes or entries")]
    Bound { field: &'static str, max: usize },
    #[error("capture history does not contain its historical year")]
    HistoryConflict,
    #[error("capture request does not belong to its exact source target")]
    RequestTargetMismatch,
    #[error("capture digest encoding failed: {0}")]
    Encoding(String),
}
