use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use census_domain::model::{CanonicalSchool, ReviewCase};
use census_domain::school_directory::{AttestedRecord, DirectoryIndex};
use census_store::{Store, StoreError, Table};
use serde::{Deserialize, Serialize};

use super::GenerationError;

pub(crate) use census_domain::model::SCHOOL_IDENTITY_FAMILY;

mod apply;
mod generation;
mod lanes;
mod link;
mod support;

pub use generation::join_generation;
pub use lanes::{build_lane_evidence, parse_source_pairs, Overrides};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    DryRun,
    Apply,
}

impl Mode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DryRun => "dry-run",
            Self::Apply => "apply",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LaneEvidence {
    pub url: Option<String>,
    pub observed_on: Option<String>,
    pub path: String,
    pub capture_sha256: String,
    pub generation: String,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Counters {
    pub scanned: u64,
    pub linked: u64,
    pub already_linked: u64,
    pub websites: u64,
    pub review: u64,
    pub no_match: u64,
    pub refused: u64,
    pub evidence_missing: u64,
    pub missing_state: u64,
    pub exact_name: u64,
    pub core_name: u64,
    pub parenthetical: u64,
    pub parenthetical_inner: u64,
    pub alias: u64,
    pub ambiguous: u64,
    pub review_filed: u64,
    pub review_present: u64,
    #[serde(default)]
    pub co_op_members: u64,
    #[serde(default)]
    pub co_op_declined: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutcomeRow {
    pub school_id: String,
    pub name: String,
    pub city: Option<String>,
    pub state: Option<String>,
    pub outcome: String,
    pub rule: Option<String>,
    pub reason: Option<String>,
    pub detail: Option<String>,
    pub candidates: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JoinReport {
    pub mode: String,
    pub generation: String,
    pub report: String,
    pub outcomes: String,
    pub counters: Counters,
    pub lanes: BTreeMap<String, LaneEvidence>,
}

#[derive(Debug, thiserror::Error)]
pub enum JoinError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Generation(#[from] GenerationError),
    #[error("artifact {name} is not valid JSON: {source}")]
    Artifact {
        name: &'static str,
        #[source]
        source: serde_json::Error,
    },
    #[error("i/o failed for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("evidence source {provider:?} is not an admitted school-directory provider")]
    EvidenceSource { provider: String },
    #[error("evidence override {pair:?} must be SOURCE=VALUE")]
    EvidencePair { pair: String },
    #[error("evidence {kind} {pair:?} is unusable: {detail}")]
    EvidenceValue {
        kind: &'static str,
        pair: String,
        detail: String,
    },
    #[error("{detail}")]
    Invariant { detail: String },
}

struct Job<'a> {
    index: &'a DirectoryIndex,
    lanes: &'a BTreeMap<String, LaneEvidence>,
    mode: Mode,
    store: &'a Store,
    counters: Counters,
    outcomes: Vec<OutcomeRow>,
    pending: Vec<CanonicalSchool>,
    existing_cases: BTreeSet<String>,
    pending_cases: Vec<ReviewCase>,
    association: Option<String>,
    association_problem: Option<String>,
    attested: Vec<AttestedRecord>,
}

pub fn process(
    store: &Store,
    index: &DirectoryIndex,
    lanes: &BTreeMap<String, LaneEvidence>,
    mode: Mode,
) -> Result<(Counters, Vec<OutcomeRow>), JoinError> {
    let mut job = Job::new(index, lanes, mode, store)?;
    store
        .snapshot()
        .for_each_merged(Table::Schools, |school: CanonicalSchool| job.visit(&school))?;
    Ok(job.finish()?)
}

#[cfg(test)]
#[path = "join_tests.rs"]
mod tests;
