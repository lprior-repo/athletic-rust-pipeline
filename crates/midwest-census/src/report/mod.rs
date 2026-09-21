//! Measured census report. Every number is computed from consolidated entity logs — never asserted
//! in prose — so the acceptance answers are reproducible from the store alone.
//!
//! The report deliberately deserializes the **canonical** entity types rather than mirroring their
//! wire shape: a hand-written mirror silently drifts from the model and would report on fields that
//! no longer exist.

use anyhow::{Context, Result};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, Evidence,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader};
use std::path::Path;

// The verbatim `tests` module resolves `Store` and `Table` through `use super::*`, exactly as the
// net split feeds its tests module from `mod.rs`.
#[cfg(test)]
use crate::store::{Store, Table};

mod notes;
mod projection;
mod rows;
mod tables;
mod writer;

pub use projection::build_census;
pub use writer::write_census;

/// Stream a JSONL entity log, tolerating a truncated tail from an interrupted run.
///
/// This reads a *materialized snapshot*: the census itself reads the store through
/// [`Store::scan`](crate::store::Store::scan), and this survives for callers that re-read an
/// export they just wrote. Every iteration consumes one line of a finite file, so the loop
/// terminates on the line count.
pub fn read_rows<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<Vec<T>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = std::fs::File::open(path).with_context(|| format!("open {}", path.display()))?;
    let mut rows = Vec::new();
    let mut unparseable = 0usize;
    for (index, line) in BufReader::new(file).lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<T>(trimmed) {
            Ok(row) => rows.push(row),
            Err(error) => {
                unparseable = unparseable.saturating_add(1);
                anyhow::ensure!(
                    unparseable <= 1,
                    "unparseable row {} in {}: {error}",
                    index.saturating_add(1),
                    path.display()
                );
            }
        }
    }
    Ok(rows)
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct StateCensus {
    pub state: String,
    pub schools: usize,
    pub athletes: usize,
    pub class_of_2027: usize,
    pub class_of_2027_boys: usize,
    pub class_of_2027_girls: usize,
    pub class_of_2027_unknown_gender: usize,
    pub class_of_2027_with_profile_url: usize,
    pub class_of_2027_with_grad_year_evidence: usize,
    pub class_of_2027_multisource: usize,
    pub class_of_2027_with_coach: usize,
    pub class_of_2027_with_coach_email: usize,
    pub coaches: usize,
    pub coaches_with_email: usize,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct SportsBreakdown {
    pub indoor_only: usize,
    pub outdoor_only: usize,
    pub cross_country_only: usize,
    pub multi_sport: usize,
    pub none: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderCoverage {
    /// Source namespaces observed on class-of-2027 athletes (`milesplit_athlete`, …).
    pub namespaces: BTreeMap<String, usize>,
    /// Class-of-2027 athletes reachable through ≥2 distinct source namespaces.
    pub multisource_athletes: usize,
    /// Class-of-2027 athletes whose Athletic.net profile URL is already known without any Athletic.net
    /// request.
    pub athletic_net_urls_known: usize,
    /// `SourceRef::id` values that appear in observed-grade evidence for class-of-2027 athletes.
    pub grade_evidence_sources: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Census {
    pub generated_on: String,
    pub store_dir: String,
    /// `all_sources` or `core` — see [`Scope`].
    pub scope: String,
    pub totals: StateCensus,
    pub by_state: BTreeMap<String, StateCensus>,
    pub athletes_by_grad_year: BTreeMap<String, usize>,
    pub class_of_2027_sports: SportsBreakdown,
    pub providers: ProviderCoverage,
    pub coach_roles: BTreeMap<String, usize>,
    pub coach_sports: BTreeMap<String, usize>,
    pub coach_sources: BTreeMap<String, usize>,
    /// Meet coverage, including how many meets already name their Athletic.net counterpart.
    pub meets: MeetCoverage,
    pub duplicate_school_names: usize,
    pub notes: Vec<String>,
}

/// Meet-table coverage. `with_athletic_net_id` counts meets whose source identities include an
/// Athletic.net meet id, i.e. meets that can be requested from Athletic.net directly by id instead
/// of being discovered by enumeration.
#[derive(Debug, Clone, Default, Serialize)]
pub struct MeetCoverage {
    pub total: usize,
    pub with_athletic_net_id: usize,
    pub by_state: BTreeMap<String, usize>,
    /// Timer/provider namespace slug (for example `timer_meet:live_results`) to meet count.
    pub by_provider: BTreeMap<String, usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_date: Option<String>,
}

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
    const fn file_suffix(self) -> &'static str {
        match self {
            Scope::AllSources => "",
            Scope::Core => "-core",
        }
    }
}

/// Adapter ids whose evidence does not count toward the core census.
///
/// `athleticlive_*` is the Athletic.net mirror (meet index, athlete rows) and `athleticnet` is the
/// host itself, read through the owner-authorized athlete-bio adapter. A core entity must be
/// reachable without any of them, so their evidence is ignored while the core filter runs.
pub const NON_CORE_SOURCE_IDS: [&str; 3] = [
    "athleticlive_athletes",
    "athleticlive_meets_csv",
    "athleticnet",
];

/// Entity tables a non-core adapter can populate.
pub trait CoreScoped {
    fn evidence(&self) -> &[Evidence];

    fn evidence_mut(&mut self) -> &mut Vec<Evidence>;

    /// Drop grade observations that came from a non-core source. No-op where a table has none.
    fn drop_non_core_observations(&mut self) {}

    /// Drop identities minted from a non-core namespace. No-op where a table has none.
    fn drop_non_core_identities(&mut self) {}
}

/// True when `id` is an adapter that belongs to the platform's own core.
pub fn is_core_source(id: &str) -> bool {
    !NON_CORE_SOURCE_IDS.contains(&id)
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
        row.evidence_mut()
            .retain(|evidence| is_core_source(&evidence.source.id));
        row.drop_non_core_observations();
        row.drop_non_core_identities();
    }
    rows.retain(|row| !row.evidence().is_empty());
    before.saturating_sub(rows.len())
}

#[cfg(test)]
mod tests;
