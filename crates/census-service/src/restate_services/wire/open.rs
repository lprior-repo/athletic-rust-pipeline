//! The open-work family: what the durable run still owes, as `Census/open-work` reports it.
//!
//! Separate from the rest of the wire protocol because it is the one reply the service cannot
//! assemble from the store. A jurisdiction sweep and a source object are units of the workflow, and
//! their records live in the Restate objects that ran them — not in an artifact — so these counts
//! are read from the journal instead.

use census_domain::UsJurisdiction;
use serde::{Deserialize, Serialize};

use crate::census::JurisdictionStages;

/// `Census/open-work`: read the open work the durable run owns, from the objects that own it.
///
/// The store cannot answer these two counts. A jurisdiction sweep and a source object are units of
/// the workflow, and their state lives in the Restate objects that ran them, so the caller names the
/// run by season and revision — the same identity those objects are keyed by — and the source
/// objects it owns, because object keys are the caller's to choose and the service cannot enumerate
/// them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenWorkRequest {
    /// Season start year: 2026 is the 2026-27 school year.
    pub season: i16,
    /// Run revision. A jurisdiction object's key is `<kind>:<state>:<season>:<revision>`.
    #[serde(default = "first_revision")]
    pub revision: u32,
    /// Ingest object keys to read. Empty means the caller named none, and the source-object count
    /// stays unmeasured rather than passing as a zero.
    #[serde(default)]
    pub source_objects: Vec<String>,
}

/// The revision a run defaults to, matching the workflow commands' `--revision`.
fn first_revision() -> u32 {
    1
}

/// One jurisdiction's stage record, and whether its object answered at all.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionOpen {
    pub jurisdiction: UsJurisdiction,
    /// The object's key, so an operator can look the row up in the journal without recomputing it.
    pub identity: String,
    pub stages: JurisdictionStages,
    /// A read that failed is not a zero: the row is reported, counted as owing, and flagged here.
    pub unreadable: bool,
}

/// One source object's state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceObjectOpen {
    pub endpoint: String,
    pub observations: u64,
    pub windows: u64,
    pub unreadable: bool,
}

/// The open work the durable run reports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenWorkReply {
    pub season: String,
    pub revision: u32,
    /// Owed jurisdiction sweeps. `None` when not one jurisdiction object could be read.
    pub jurisdiction_sweeps: Option<u64>,
    /// Source objects whose acquisition has no terminal state yet. `None` when the caller named
    /// none, or when not one of them could be read.
    pub source_objects: Option<u64>,
    /// Of [`Self::endpoints`]: the endpoints that finished their walk without appending a row, by
    /// name. Defaulted rather than required, because replies recorded before this field existed are
    /// still replayed — and a replayed reply that cannot be read is a run that cannot be measured.
    #[serde(default)]
    pub silent_sources: Vec<String>,
    /// Every jurisdiction in the run scope, owed or not, in scope order.
    pub jurisdictions: Vec<JurisdictionOpen>,
    /// Every source object the caller named, in request order.
    pub endpoints: Vec<SourceObjectOpen>,
}
