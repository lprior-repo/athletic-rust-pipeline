#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DomainError {
    #[error("{field} is outside its permitted range")]
    OutOfRange { field: &'static str },
    #[error("{field} is not supported")]
    Unsupported { field: &'static str },
}
