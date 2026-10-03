pub use census_domain::{JurisdictionBucket, MeetState};
use serde::ser::SerializeMap;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    #[error(transparent)]
    Store(#[from] census_store::StoreError),
    #[error("i/o failed for {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("saving the workbook to {path}: {source}")]
    Xlsx {
        path: std::path::PathBuf,
        #[source]
        source: rust_xlsxwriter::XlsxError,
    },
    #[error("unparseable row {line} in {path}: {source}")]
    Decode {
        path: std::path::PathBuf,
        line: usize,
        #[source]
        source: serde_json::Error,
    },
    #[error("{detail}")]
    Invariant { detail: String },
    #[error("publication failed: {operation}; temporary cleanup also failed: {cleanup}")]
    Cleanup {
        operation: Box<ReportError>,
        cleanup: Box<ReportError>,
    },
}

pub type ReportResult<T, E = ReportError> = std::result::Result<T, E>;

pub(crate) fn io_error(path: &Path, source: std::io::Error) -> ReportError {
    ReportError::Io {
        path: path.to_path_buf(),
        source,
    }
}

pub(crate) fn xlsx_error(path: &Path, source: rust_xlsxwriter::XlsxError) -> ReportError {
    ReportError::Xlsx {
        path: path.to_path_buf(),
        source,
    }
}

#[cfg(test)]
use census_store::{Store, Table};

mod core_scope;
mod coverage;
mod derivation;
mod notes;
mod projection;
mod rows;
mod tables;
mod writer;

pub use coverage::{
    coverage_report, CoverageGap, CoverageReport, CoverageTotals, GapClass, JurisdictionCoverage,
    UNKNOWN_JURISDICTION,
};
pub(crate) use derivation::cohort_candidates;
pub use derivation::Derivation;
pub use projection::build_census;
pub use writer::write_census;

pub use census_domain::core_scope::{is_core_source, NON_CORE_SOURCE_IDS};
pub use core_scope::{is_core_evidenced, retain_core, retain_core_row, CoreScoped, Scope};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowLabel {
    Jurisdiction(JurisdictionBucket),
    Total,
}

impl Default for RowLabel {
    fn default() -> Self {
        Self::Jurisdiction(JurisdictionBucket::Unplaced)
    }
}

impl RowLabel {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Jurisdiction(bucket) => bucket.code(),
            Self::Total => "TOTAL",
        }
    }
}

impl Serialize for RowLabel {
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
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct StateCensus {
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
    pub namespaces: BTreeMap<String, usize>,
    pub multisource_athletes: usize,
    pub athletic_net_urls_known: usize,
    pub grade_evidence_sources: BTreeMap<String, usize>,
}

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
    pub scope: String,
    pub totals: StateCensus,
    #[serde(serialize_with = "by_jurisdiction_code")]
    pub by_state: BTreeMap<JurisdictionBucket, StateCensus>,
    pub athletes_by_grad_year: BTreeMap<String, usize>,
    pub class_of_2027_sports: SportsBreakdown,
    pub providers: ProviderCoverage,
    pub coach_roles: BTreeMap<String, usize>,
    pub coach_sports: BTreeMap<String, usize>,
    pub coach_sources: BTreeMap<String, usize>,
    pub meets: MeetCoverage,
    pub duplicate_school_names: usize,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct MeetCoverage {
    pub total: usize,
    pub with_athletic_net_id: usize,
    #[serde(serialize_with = "meet_state_code")]
    pub by_state: BTreeMap<MeetState, usize>,
    pub by_provider: BTreeMap<String, usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_date: Option<String>,
}

#[cfg(test)]
mod tests;
