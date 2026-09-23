//! Measured census report. Every number is computed from consolidated entity logs — never asserted
//! in prose — so the acceptance answers are reproducible from the store alone.
//!
//! The report deliberately deserializes the **canonical** entity types rather than mirroring their
//! wire shape: a hand-written mirror silently drifts from the model and would report on fields that
//! no longer exist.

pub use census_domain::{JurisdictionBucket, MeetState};
use serde::ser::SerializeMap;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Report, bests and workbook failures.
#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    /// The underlying store scan failed.
    #[error(transparent)]
    Store(#[from] census_store::StoreError),
    /// A report or workbook file operation failed.
    #[error("i/o failed for {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// The workbook writer rejected a sheet or cell.
    #[error("saving the workbook to {path}: {source}")]
    Xlsx {
        path: std::path::PathBuf,
        #[source]
        source: rust_xlsxwriter::XlsxError,
    },
    /// A JSONL row did not decode.
    #[error("unparseable row {line} in {path}: {source}")]
    Decode {
        path: std::path::PathBuf,
        line: usize,
        #[source]
        source: serde_json::Error,
    },
    /// An aggregation counter at the end of its range.
    #[error("counter overflow")]
    CounterOverflow,
    /// An invariant the report relies on was violated: a bug, not external input.
    #[error("{detail}")]
    Invariant { detail: String },
}

/// Result alias for report code.
pub type ReportResult<T> = std::result::Result<T, ReportError>;

/// Attach the path an i/o failure came from, so a typed [`ReportError::Io`] can name it.
pub(crate) fn io_error(path: &Path, source: std::io::Error) -> ReportError {
    ReportError::Io {
        path: path.to_path_buf(),
        source,
    }
}

/// Attach the workbook path a `rust_xlsxwriter` rejection belongs to.
pub(crate) fn xlsx_error(path: &Path, source: rust_xlsxwriter::XlsxError) -> ReportError {
    ReportError::Xlsx {
        path: path.to_path_buf(),
        source,
    }
}

// The verbatim `tests` module resolves `Store` and `Table` through `use super::*`, exactly as the
// net split feeds its tests module from `mod.rs`.
#[cfg(test)]
use census_store::{Store, Table};

mod core_scope;
mod coverage;
mod notes;
mod projection;
mod rows;
mod tables;
mod writer;

pub use coverage::{
    coverage_report, CoverageGap, CoverageReport, CoverageTotals, GapClass, JurisdictionCoverage,
    UNKNOWN_JURISDICTION,
};
pub(crate) use coverage::{jurisdiction_of, school_state_index};
pub use projection::build_census;
/// The run-scope predicate every consumer of a store row shares: `true` for the jurisdictions
/// `UsJurisdiction::CENSUS_SCOPE` names and for the unplaced row, `false` for every other
/// jurisdiction a store may still hold rows for.
pub(crate) use projection::in_run_scope;
pub use writer::write_census;

pub use core_scope::{is_core_source, retain_core, CoreScoped, Scope, NON_CORE_SOURCE_IDS};

/// The label one published row prints: where it is bucketed, or the grand total.
///
/// A bucket alone cannot describe the totals row, and the totals row is published inside the same
/// document (`totals.state == "TOTAL"`), so the label is its own value rather than a string a caller
/// could fill with anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowLabel {
    /// A per-jurisdiction row: the state or DC it buckets, or the unplaced row.
    Jurisdiction(JurisdictionBucket),
    /// The row that sums every bucket.
    Total,
}

impl Default for RowLabel {
    /// A row that has not been labelled reads as the unplaced bucket, never as a jurisdiction and
    /// never as the grand total.
    fn default() -> Self {
        Self::Jurisdiction(JurisdictionBucket::Unplaced)
    }
}

impl RowLabel {
    /// The label the row prints as: the bucket's code, or `TOTAL`.
    pub const fn code(self) -> &'static str {
        match self {
            Self::Jurisdiction(bucket) => bucket.code(),
            Self::Total => "TOTAL",
        }
    }
}

impl Serialize for RowLabel {
    /// Serializes as the printed label (`"WI"`, `"UNKNOWN"`, `"TOTAL"`), so the published JSON keeps
    /// the vocabulary its readers already parse.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

impl From<JurisdictionBucket> for RowLabel {
    fn from(bucket: JurisdictionBucket) -> Self {
        Self::Jurisdiction(bucket)
    }
}

impl fmt::Display for RowLabel {
    /// Displays [`Self::code`], the form the published row carries.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct StateCensus {
    /// The row's own label: its jurisdiction bucket, or `TOTAL` for the summed row.
    pub state: RowLabel,
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

/// Publishes `by_state` in jurisdiction-code order.
///
/// Buckets order by the domain's declaration order, which is what the coverage rows document;
/// the published map keeps the code order every release has written, so diffing two `report.json`
/// files shows rows whose numbers moved instead of every key reshuffling.
fn by_jurisdiction_code<S>(
    map: &BTreeMap<JurisdictionBucket, StateCensus>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let mut rows: Vec<(&JurisdictionBucket, &StateCensus)> = map.iter().collect();
    rows.sort_by_key(|(bucket, _)| bucket.code());
    let mut entries = serializer.serialize_map(Some(rows.len()))?;
    for (bucket, row) in rows {
        entries.serialize_entry(bucket, row)?;
    }
    entries.end()
}

/// Publishes the meet-state map in the same code order, with the unresolved label (`??`) first,
/// which is where a byte-ordered map of the old `String` keys put it.
fn meet_state_code<S>(map: &BTreeMap<MeetState, usize>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let mut rows: Vec<(&MeetState, &usize)> = map.iter().collect();
    rows.sort_by_key(|(state, _)| state.code());
    let mut entries = serializer.serialize_map(Some(rows.len()))?;
    for (state, count) in rows {
        entries.serialize_entry(state, count)?;
    }
    entries.end()
}

#[derive(Debug, Clone, Serialize)]
pub struct Census {
    pub generated_on: String,
    pub store_dir: String,
    /// `all_sources` or `core` — see [`Scope`].
    pub scope: String,
    pub totals: StateCensus,
    /// One row per configured jurisdiction, plus [`UNKNOWN_JURISDICTION`] for rows no school
    /// placed. Every key is present whatever the store holds, so a state with no observations
    /// publishes zeros instead of vanishing from the sheet and the per-state CSV.
    #[serde(serialize_with = "by_jurisdiction_code")]
    pub by_state: BTreeMap<JurisdictionBucket, StateCensus>,
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
    /// Counts per [`MeetState`]: the USPS code, or the store's unresolved sentinel (`??`). Meets
    /// carry the meet-state label rather than the school bucket, so an unplaced venue keeps the
    /// label every published artifact already prints for it.
    #[serde(serialize_with = "meet_state_code")]
    pub by_state: BTreeMap<MeetState, usize>,
    /// Timer/provider namespace slug (for example `timer_meet:live_results`) to meet count.
    pub by_provider: BTreeMap<String, usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_date: Option<String>,
}

#[cfg(test)]
mod tests;
