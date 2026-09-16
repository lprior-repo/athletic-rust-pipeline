#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DomainError {
    #[error("{field} is empty")]
    Empty { field: &'static str },
    #[error("{field} exceeds {limit} bytes")]
    TooLong { field: &'static str, limit: usize },
    #[error("{field} has an invalid format")]
    InvalidFormat { field: &'static str },
    #[error("{field} is outside its permitted range")]
    OutOfRange { field: &'static str },
    #[error("independent evidence is missing")]
    MissingEvidence,
    #[error("independent evidence is contradictory")]
    ContradictoryEvidence,
    #[error("the three-retry budget is exhausted")]
    RetryExhausted,
    #[error("{field} is not supported")]
    Unsupported { field: &'static str },
}
