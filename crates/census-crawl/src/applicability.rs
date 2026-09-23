//! Which registered sources a census run may plan for one jurisdiction, and the research each row
//! rests on.
//!
//! Why this module exists: a jurisdiction fan-out decides which adapters to run *before* any adapter
//! runs, and the answer is neither "the whole registry" nor a per-jurisdiction list kept in prose.
//! `ihsa` is Illinois, `wayzata` is a Minnesota/Iowa/Wisconsin timer, `tfrrs`'s high-school depth is
//! three state instances, and the two national platforms answer for every state — facts that live in
//! the research corpus and nowhere in the adapters themselves. A plan that ignores them spends
//! requests the research already refuted; a plan that keeps them in prose drifts from the code that
//! has to honour them.
//!
//! The rows are the research's conclusions, not a re-derivation. Each cites its reports (`[01]`-`[29]`
//! from `research/midwest-source-program/synthesis/05-source-matrix.md`, which is where
//! `data/source-coverage-matrix.csv`'s `report_ref` column points, and the lane ids §0a names), and
//! the confidence the research recorded is quoted inside [`Applicability::evidence`] rather than
//! mirrored as a field: one row aggregates several state rows whose confidences differ, and the
//! research's own words are the honest carrier for them.
//!
//! Two rules keep the table honest:
//!
//! * A jurisdiction/source pair with no report behind it is a guess, and a guess is not a row. A
//!   jurisdiction no row evidences plans nothing — a real answer, which the fan-out must read as
//!   "plan no source" rather than "plan everything".
//! * Every row also states why the jurisdictions it omits are excluded ([`Applicability::refusal`]),
//!   so "why does Texas plan no KSHSAA directory" is answered where the inclusion rule lives. §3b of
//!   the matrix (routes tested and rejected — do not re-attempt without a decision) is where a
//!   reviewer goes for the rejected *routes*, which no longer appear here at all, e.g. `live.pttiming`
//!   automated use (ToU, MO/MI) and `mtecresults` (authenticated).

mod table;

use crate::registry::{bulk_first, SourceDescriptor};
use census_domain::UsJurisdiction;

use table::TABLE;

/// One registered adapter's applicability across the census scope, and the half of the rule that
/// says why the other jurisdictions are left out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Applicability {
    /// The adapter's slug, exactly as [`crate::descriptor`] resolves it.
    pub slug: &'static str,
    /// The in-scope jurisdictions the research evidences this adapter for: a subset of
    /// [`UsJurisdiction::CENSUS_SCOPE`], never a jurisdiction a census run may not touch.
    pub jurisdictions: &'static [UsJurisdiction],
    /// The reports behind the row, in the corpus's citation form, quoting the confidence the research
    /// recorded where it differs per state.
    pub evidence: &'static str,
    /// Why the jurisdictions this row omits are excluded. Never empty: a source that needs no
    /// exclusion says that instead.
    pub refusal: &'static str,
}

/// One row per registered adapter: the research corpus's applicability table, in slug order.
pub fn table() -> &'static [Applicability] {
    &TABLE
}

/// The sources a census run may plan for one jurisdiction, in [`bulk_first`] planning order.
///
/// Total over [`UsJurisdiction::ALL`]: a jurisdiction outside the census scope, or one no row
/// evidences, answers with an empty plan rather than panicking or falling back to the whole registry.
/// Order comes from [`bulk_first`] and not from this table, so a plan's request budget order and this
/// table's reading order cannot drift apart.
pub fn applicable_sources(jurisdiction: UsJurisdiction) -> Vec<&'static SourceDescriptor> {
    let slugs: Vec<&str> = TABLE
        .iter()
        .filter(|row| row.jurisdictions.contains(&jurisdiction))
        .map(|row| row.slug)
        .collect();
    bulk_first(&slugs)
}

// The test module last, as every adapter does: the scan lane reads the production region as the
// lines before `#[cfg(test)]`, so code after this point would drop out of its counts.
#[cfg(test)]
#[path = "applicability/tests.rs"]
mod tests;
