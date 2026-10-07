use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use census_domain::model::{CanonicalSchool, ReviewCase};
use census_domain::school_directory::{AttestedRecord, DirectoryIndex, IdentifiedKey};
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
    #[serde(default)]
    pub captured: Vec<IdentifiedKey>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LaneSet {
    #[serde(default)]
    by_provider: BTreeMap<String, Vec<LaneEvidence>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LaneSelection {
    Missing,
    Unattributed { candidates: Vec<String> },
    Ambiguous { candidates: Vec<String> },
}

impl LaneSet {
    pub(super) fn of(mut by_provider: BTreeMap<String, Vec<LaneEvidence>>) -> Self {
        for lanes in by_provider.values_mut() {
            for lane in lanes.iter_mut() {
                lane.captured.sort();
            }
        }
        Self { by_provider }
    }

    pub fn tokens(&self) -> impl Iterator<Item = &String> {
        self.by_provider.keys()
    }

    pub fn lanes(&self, provider: &str) -> &[LaneEvidence] {
        self.by_provider.get(provider).map_or(&[], Vec::as_slice)
    }

    pub fn select(
        &self,
        provider: &str,
        key: &IdentifiedKey,
    ) -> Result<&LaneEvidence, LaneSelection> {
        let lanes = self.lanes(provider);
        let [single] = lanes else {
            let candidates: Vec<&LaneEvidence> =
                lanes.iter().filter(|lane| carries(lane, key)).collect();
            return match candidates.as_slice() {
                [lane] => Ok(lane),
                [] => Err(LaneSelection::Unattributed {
                    candidates: lanes.iter().map(|lane| lane.path.clone()).collect(),
                }),
                many => Err(LaneSelection::Ambiguous {
                    candidates: many.iter().map(|lane| lane.path.clone()).collect(),
                }),
            };
        };
        Ok(single)
    }
}

fn carries(lane: &LaneEvidence, key: &IdentifiedKey) -> bool {
    lane.captured.binary_search(key).is_ok()
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Counters {
    pub scanned: u64,
    pub linked: u64,
    pub already_linked: u64,
    #[serde(default)]
    pub backfilled: u64,
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
    pub lanes: LaneSet,
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
    lanes: &'a LaneSet,
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
    lanes: &LaneSet,
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
