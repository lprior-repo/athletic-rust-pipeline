use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::census::{MeetCensus, Revision, StateProgress};

pub(super) mod ingest;

/// The ingest family, re-exported so this module remains the one wire-protocol surface to import
/// from. The types themselves are defined in the private `ingest` module.
pub use ingest::{
    EndpointObservation, IngestReply, IngestRequest, IngestState, SweepReport, SweepRequest,
    WindowRequest,
};

pub(super) mod open;

/// The open-work family, re-exported for the same reason as the ingest family above. The types
/// themselves are defined in the private `open` module.
pub use open::{JurisdictionOpen, OpenWorkReply, OpenWorkRequest, SourceObjectOpen};

pub(super) mod plan;

/// The source-plan family, re-exported for the same reason as the families above. The types
/// themselves are defined in the private `plan` module.
pub use plan::{RefusedSource, SourcePlan};

pub(super) mod seal;

/// The seal family, re-exported for the same reason as the two families above: this module stays
/// the one wire-protocol surface to import from.
pub use seal::{SealItem, SealRef, SealReply, SealRequest};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableCount {
    pub table: String,
    pub rows: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusReply {
    pub tables: Vec<TableCount>,
    pub observations: u64,
    pub bytes_on_disk: u64,
    pub today: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConsolidateRequest {
    /// Tables to consolidate; empty means every table.
    #[serde(default)]
    pub tables: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidatedTable {
    pub table: String,
    pub rows: usize,
    /// Consumer mailboxes withheld from the coaches snapshot by the contact contract.
    pub emails_withheld: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidateReply {
    pub tables: Vec<ConsolidatedTable>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReportRequest {
    /// `core` (default) or `all_sources`.
    #[serde(default)]
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportReply {
    pub scope: String,
    pub generated_on: String,
    /// Totals as the census document publishes them; the per-state breakdown stays in the JSON/CSV.
    pub totals: Value,
    pub json_path: String,
    pub csv_path: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BestsRequest {
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub grad_year: Option<i16>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BestsReply {
    pub cohort: String,
    pub rows: usize,
    pub jsonl: String,
    pub csv: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkbookRequest {
    #[serde(default)]
    pub grad_year: Option<i16>,
    #[serde(default)]
    pub limit: Option<usize>,
    /// Scope the best-results sidecars are reduced over (`core` or `all_sources`); `core` when
    /// absent, the same default the operator-facing reductions use.
    #[serde(default)]
    pub scope: Option<String>,
    /// Directory the workbook is written into; the store's own `out/` when absent.
    #[serde(default)]
    pub out: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkbookReply {
    pub path: String,
    pub grad_year: Option<i16>,
}

/// What one jurisdiction's census asks for. The object key is the jurisdiction identity
/// (`jurisdiction:<state>:<season>:<revision>`), so the request repeats those three fields only so
/// the handler can refuse a request routed to the wrong key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionRequest {
    pub jurisdiction: UsJurisdiction,
    pub season: SchoolYear,
    pub revision: Revision,
    /// Bypass cached bodies for this run.
    #[serde(default)]
    pub refresh: bool,
    /// Rosters per jurisdiction; absent means every team the index lists.
    #[serde(default)]
    pub limit_per_state: Option<usize>,
    /// Rosters fetched concurrently inside this jurisdiction.
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    /// Collection date stamped on the evidence this run writes; absent means today.
    #[serde(default)]
    pub observed_on: Option<String>,
}

fn default_concurrency() -> usize {
    4
}

impl NationalRequest {
    /// The per-jurisdiction request one fan-out call carries. The national request holds the shared
    /// knobs — season, revision, refresh, roster ceiling, concurrency, collection date — and this
    /// projects them onto one state.
    pub fn for_jurisdiction(&self, jurisdiction: UsJurisdiction) -> JurisdictionRequest {
        JurisdictionRequest {
            jurisdiction,
            season: self.season,
            revision: self.revision,
            refresh: self.refresh,
            limit_per_state: self.limit_per_state,
            concurrency: self.concurrency,
            observed_on: self.observed_on.clone(),
        }
    }
}

/// One completed stage: how many records it produced and when it finished.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageOutcome {
    pub records: usize,
    pub at: String,
}

/// A jurisdiction object's whole durable state.
///
/// One value, written whole: the object key is the jurisdiction identity, and a partially updated
/// jurisdiction — teams counted but rosters not recorded, or the reverse — must not be able to
/// exist, because a resumed run reads this to decide which stages it still owes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JurisdictionState {
    /// The identity this state belongs to; empty before the first run.
    #[serde(default)]
    pub identity: String,
    /// The sources this jurisdiction's run carries. Built once, before the first stage, and kept:
    /// a re-invocation resumes the plan it started with rather than deriving a second one. Absent
    /// on a state journaled before the plan existed, which is why the object computes it whenever
    /// it is missing.
    #[serde(default)]
    pub plan: Option<SourcePlan>,
    #[serde(default)]
    pub teams: Option<StageOutcome>,
    /// The roster walk's measured outcome, including its cohort counts and per-team errors.
    #[serde(default)]
    pub rosters: Option<StateProgress>,
    #[serde(default)]
    pub consolidated: Option<Vec<ConsolidatedTable>>,
    /// The meet census's measured outcome. Absent on a state journaled before the stage existed,
    /// which is why the stage keys on this field rather than on the invocation's stage list.
    #[serde(default)]
    pub meets: Option<MeetCensus>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// The result of one `run`: the stages this invocation executed, and the state it left behind.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionReport {
    pub identity: String,
    pub jurisdiction: UsJurisdiction,
    /// The plan the run carried: the applicable sources this machine may sweep, and the ones it
    /// refuses by name. Read beside `stages_run` — the plan is the declared work, the stages are
    /// what ran — and note that nothing in this crate yet reconciles the two.
    #[serde(default)]
    pub plan: SourcePlan,
    /// Stages executed now, in order. Empty means every stage was already complete.
    pub stages_run: Vec<String>,
    pub teams: usize,
    pub rosters: StateProgress,
    pub consolidated: Vec<ConsolidatedTable>,
    pub meets: MeetCensus,
    pub completed_at: String,
}

/// The national run's request. `jurisdictions` empty means all fifty states and the District of
/// Columbia, in declaration order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NationalRequest {
    pub season: SchoolYear,
    pub revision: Revision,
    #[serde(default)]
    pub jurisdictions: Vec<UsJurisdiction>,
    #[serde(default)]
    pub refresh: bool,
    #[serde(default)]
    pub limit_per_state: Option<usize>,
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    #[serde(default)]
    pub observed_on: Option<String>,
}

/// One jurisdiction's row in the national report.
///
/// The walk's three outcomes are separate on purpose. `rosters_done` is what this traversal
/// fetched, `rosters_skipped` is what the journal already held, and `rosters_owed` is what the run
/// left unfinished — a state whose host refused requests (§69) stops the walk with most of its
/// index still owed. Without the last two an operator reading a blocked state sees a small state.
///
/// Both are `Option` because a report written by an earlier revision does not carry them: a missing
/// denominator must read as unknown, never as zero.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionSummary {
    pub jurisdiction: UsJurisdiction,
    pub identity: String,
    pub teams: usize,
    pub rosters_done: usize,
    pub rosters_skipped: usize,
    /// Rosters this run left unfinished: no journal entry, no fetched page.
    #[serde(default)]
    pub rosters_owed: Option<usize>,
    /// The host refused at least one roster with HTTP 403/429, which ends the state's requests.
    #[serde(default)]
    pub blocked: Option<bool>,
    pub athletes: usize,
    pub class_of_2027: usize,
}

/// A jurisdiction whose run failed. The national run keeps going: one jurisdiction's source outage
/// is a row in this list, never a failed national run (§69).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NationalFailure {
    pub jurisdiction: UsJurisdiction,
    pub identity: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NationalReport {
    pub season: SchoolYear,
    pub revision: Revision,
    pub jurisdictions: Vec<JurisdictionSummary>,
    pub failures: Vec<NationalFailure>,
    pub teams_total: usize,
    pub athletes_total: usize,
    pub class_of_2027_total: usize,
    pub today: String,
}
