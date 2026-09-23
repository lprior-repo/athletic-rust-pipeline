//! The report's scope: which rows count toward the platform's core.
//!
//! [`Scope`] names the two views the report publishes, and [`CoreScoped`] with [`retain_core`] is the
//! filter that produces the core view. Which adapter ids are outside the core is the domain's answer
//! ([`census_domain::core_scope`]); this module is the one that applies it to report rows. Kept out of
//! `mod.rs` so the scope contract reads on its own, away from the census document it selects rows for.

use census_domain::is_core_source;
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, Evidence,
};

/// Report scope.
///
/// The platform's **core** deliberately excludes Athletic.net and the AthleticLIVE derivative: the
/// objective requires the core to work, and be measurable, with those adapters never registered.
/// Every core number therefore has to be reachable from association, MileSplit, official-artifact,
/// timer, or school-site evidence alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Every consolidated row, including AthleticLIVE enrichment.
    AllSources,
    /// Only entities with at least one evidence source that is not Athletic.net-derived.
    Core,
}

impl Scope {
    pub const fn as_str(self) -> &'static str {
        match self {
            Scope::AllSources => "all_sources",
            Scope::Core => "core",
        }
    }

    /// `""` or `"-core"`, appended to `report.json` / `census-by-state.csv`.
    pub(super) const fn file_suffix(self) -> &'static str {
        match self {
            Scope::AllSources => "",
            Scope::Core => "-core",
        }
    }
}

/// Entity tables a non-core adapter can populate.
pub trait CoreScoped {
    fn evidence(&self) -> &[Evidence];

    fn evidence_mut(&mut self) -> &mut Vec<Evidence>;

    /// Drop grade observations that came from a non-core source. No-op where a table has none.
    fn drop_non_core_observations(&mut self) {}

    /// Drop identities minted from a non-core namespace. No-op where a table has none.
    fn drop_non_core_identities(&mut self) {}
}

impl CoreScoped for CanonicalAthlete {
    fn evidence(&self) -> &[Evidence] {
        &self.evidence
    }

    fn evidence_mut(&mut self) -> &mut Vec<Evidence> {
        &mut self.evidence
    }

    fn drop_non_core_observations(&mut self) {
        self.observed_grades
            .retain(|observation| is_core_source(&observation.source.id));
    }

    fn drop_non_core_identities(&mut self) {
        self.source_identities
            .retain(|identity| identity.namespace.is_core());
    }
}

impl CoreScoped for CanonicalMeet {
    fn evidence(&self) -> &[Evidence] {
        &self.evidence
    }

    fn evidence_mut(&mut self) -> &mut Vec<Evidence> {
        &mut self.evidence
    }

    fn drop_non_core_identities(&mut self) {
        self.source_identities
            .retain(|identity| identity.namespace.is_core());
    }
}

/// An event carries evidence and per-source labels, and no identities: the default no-ops cover
/// everything but the evidence filter, which is the rule every other table follows.
impl CoreScoped for CanonicalEvent {
    fn evidence(&self) -> &[Evidence] {
        &self.evidence
    }

    fn evidence_mut(&mut self) -> &mut Vec<Evidence> {
        &mut self.evidence
    }
}

/// A performance carries evidence, a bare grade, and a provider-local key; only the evidence can name
/// the adapter that produced the row.
impl CoreScoped for CanonicalPerformance {
    fn evidence(&self) -> &[Evidence] {
        &self.evidence
    }

    fn evidence_mut(&mut self) -> &mut Vec<Evidence> {
        &mut self.evidence
    }
}

/// Reduce rows to what the core could know on its own.
///
/// Non-core evidence, grade observations, and identities are removed first, exactly as they would be
/// absent had the non-core adapters never been registered; a row left with no evidence at all is
/// then dropped. Returns the number of dropped rows.
pub fn retain_core<T: CoreScoped>(rows: &mut Vec<T>) -> usize {
    let before = rows.len();
    for row in rows.iter_mut() {
        retain_core_row(row);
    }
    rows.retain(|row| !row.evidence().is_empty());
    before.saturating_sub(rows.len())
}

/// Apply that same reduction to one row in hand, answering whether it survives.
///
/// The streaming reader cannot drop a row before reading it, so this is the per-row half of
/// [`retain_core`]: one definition, so a merged table and a streamed one cannot disagree about what
/// the core scope holds.
pub fn retain_core_row<T: CoreScoped>(row: &mut T) -> bool {
    row.evidence_mut()
        .retain(|evidence| is_core_source(&evidence.source.id));
    row.drop_non_core_observations();
    row.drop_non_core_identities();
    !row.evidence().is_empty()
}
