//! Domain failures of the pure model crate: every variant is enumerable and names the value it
//! rejected, never a rendered string.
//!
//! `field` names the value in the ubiquitous language (`school_year`, `gender`, `grad_year`), not
//! the wire field, so one failure reads the same in every adapter that can raise it.

/// A canonical value could not be formed from external input.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DomainError {
    /// A required value was absent.
    #[error("{field} is missing")]
    Missing { field: &'static str },
    /// An empty string where a value is required.
    #[error("{field} is empty")]
    Empty { field: &'static str },
    /// A value exceeded its byte or element ceiling.
    #[error("{field} exceeds {limit} bytes")]
    TooLong { field: &'static str, limit: usize },
    /// A value did not match its documented shape.
    #[error("{field} has an invalid format")]
    InvalidFormat { field: &'static str },
    /// A value parsed but lies outside its permitted range.
    #[error("{field} is outside its permitted range")]
    OutOfRange { field: &'static str },
    /// Independent evidence for a grade or identity claim is absent.
    #[error("independent evidence is missing")]
    MissingEvidence,
    /// Independent evidence for a grade or identity claim contradicts itself.
    #[error("independent evidence is contradictory")]
    ContradictoryEvidence,
    /// A value names something this pipeline does not model.
    #[error("{field} is not supported")]
    Unsupported { field: &'static str },
}

/// Result alias: `census_domain` code defaults to the domain failure.
pub type Result<T, E = DomainError> = std::result::Result<T, E>;
