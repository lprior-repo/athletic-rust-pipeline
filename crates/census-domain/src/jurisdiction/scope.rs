//! The census run scope and the typed rejection that enforces it.
//!
//! `CENSUS_SCOPE` is accessible as [`super::table::UsJurisdiction::CENSUS_SCOPE`].
//!
//! `OutsideCensusScope` is the typed error produced when a jurisdiction falls outside the 48-state
//! run scope — Alaska and Hawaii are declared models but never admitted to a census run.

/// The typed failure of [`super::table::UsJurisdiction::require_census_scope`]: a jurisdiction a census run never
/// covers (ADR-009).
///
/// One type carrying one message, so the CLI, the workflow fan-out and the report all reject the
/// same set with the same words instead of each re-deriving the rule and the sentence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error(
    "{} is outside the census run scope: the 48 continental states plus the District of Columbia \
     (ADR-009)",
    .0.code()
)]
pub struct OutsideCensusScope(pub super::table::UsJurisdiction);
