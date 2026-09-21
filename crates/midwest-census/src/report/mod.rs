//! Measured census report. Every number is computed from consolidated entity logs — never asserted
//! in prose — so the acceptance answers are reproducible from the store alone.
//!
//! The report deliberately deserializes the **canonical** entity types rather than mirroring their
//! wire shape: a hand-written mirror silently drifts from the model and would report on fields that
//! no longer exist.

use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Report, bests and workbook failures.
#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    /// The underlying store scan failed.
    #[error(transparent)]
    Store(#[from] crate::store::StoreError),
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
use crate::store::{Store, Table};

mod core_scope;
mod notes;
mod projection;
mod rows;
mod tables;
mod writer;

pub use projection::build_census;
pub use writer::write_census;

pub use core_scope::{is_core_source, retain_core, CoreScoped, Scope, NON_CORE_SOURCE_IDS};

/// Stream a JSONL entity log, tolerating a truncated tail from an interrupted run.
///

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

#[cfg(test)]
mod tests;
