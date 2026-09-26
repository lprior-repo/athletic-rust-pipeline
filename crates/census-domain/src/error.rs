//! Domain failures of the pure model crate: every variant is enumerable and names the value it
//! rejected, never a rendered string.
//!
//! `field` names the value in the ubiquitous language (`school_year`, `gender`, `grad_year`), not
//! the wire field, so one failure reads the same in every adapter that can raise it.

/// A canonical value could not be formed from external input.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DomainError {
    /// A value parsed but lies outside its permitted range.
    #[error("{field} is outside its permitted range")]
    OutOfRange { field: &'static str },
    /// A value names something this pipeline does not model.
    #[error("{field} is not supported")]
    Unsupported { field: &'static str },
}
