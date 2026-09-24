//! The open work a census still owes, and the rule for each count that says whether it is owed.
//!
//! Three counts, from two surfaces. Owed jurisdiction sweeps and un-terminal source objects are
//! properties of the durable run, so they are read from the objects that did the work; owed cohort
//! decisions are retained store cases, read from the store like every other §70 count. This module
//! holds the part worth testing without either surface: given the states and the cases, which rows
//! are still owed.
//!
//! Nothing here counts a state it did not read. A caller that could not reach the objects leaves the
//! matching [`OpenWork`](super::OpenWork) field `None`, which the seal reports as unmeasured rather
//! than as a zero — a measurement nobody took is not a zero.

use census_domain::model::{ReviewCase, ReviewState, COHORT_DECISION_FAMILIES};
use serde::{Deserialize, Serialize};

/// The stages one jurisdiction's durable state records, and the rosters it left behind.
///
/// `Default` is "nothing recorded", deliberately: an object that could not be read, or that never
/// ran, has no stage outcome, and [`Self::terminal`] refuses a record with none.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct JurisdictionStages {
    /// Whether the team index stage recorded an outcome.
    pub teams: bool,
    /// Whether the roster walk recorded an outcome.
    pub rosters: bool,
    /// Whether the meet census recorded an outcome.
    pub meets: bool,
    /// Rosters the walk dropped unfetched after a host refused it. They stay owed, so a re-run
    /// resumes them, and a jurisdiction carrying them is not terminal even though its stages ran.
    pub owed_rosters: u64,
}

impl JurisdictionStages {
    /// A jurisdiction is terminal when every stage recorded an outcome and no roster was left
    /// unfetched.
    pub fn terminal(self) -> bool {
        self.teams && self.rosters && self.meets && self.owed_rosters == 0
    }

    /// The stages this record has no outcome for, in the order the object runs them.
    pub fn owing(self) -> Vec<&'static str> {
        let mut owing = Vec::new();
        if !self.teams {
            owing.push("teams");
        }
        if !self.rosters {
            owing.push("rosters");
        }
        if !self.meets {
            owing.push("meets");
        }
        if self.owed_rosters > 0 {
            owing.push("blocked rosters");
        }
        owing
    }
}

/// One source object's durable state: what its endpoint has accepted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceObject {
    /// The ingest object's key, which is the source namespace it serves.
    pub endpoint: String,
    /// Observations the endpoint has accepted. Zero means it has never written one.
    pub observations: u64,
    /// Windows the endpoint has been declared complete for.
    pub windows: u64,
}

impl SourceObject {
    /// A source object is terminal once it has accepted an observation, or once the walk that owns it
    /// declared a window complete.
    ///
    /// The window is the object's own record that its walk finished: `complete_window` is called only
    /// after every batch the walk produced landed, so no window closes over rows that were not
    /// appended — and a walk that produced none, because its rows were already durable from an
    /// earlier run, closes one too. Counting that as owed would hold a fully acquired source on the
    /// seal's item 2 for good.
    ///
    /// The `Sweep` asks a different question, with its own inline rule — whether an endpoint accepted
    /// anything across the windows it watched. Liveness over an observation window is not the same
    /// claim as terminality of the acquisition, so the two rules stay apart rather than shared.
    pub fn terminal(&self) -> bool {
        self.observations > 0 || self.windows > 0
    }
}

/// Count the jurisdictions whose stage record is not terminal.
pub fn owed_jurisdictions(stages: &[JurisdictionStages]) -> u64 {
    count(stages.iter().filter(|stage| !stage.terminal()).count())
}

/// Count the source objects whose acquisition has no terminal state yet.
pub fn owed_source_objects(objects: &[SourceObject]) -> u64 {
    count(objects.iter().filter(|object| !object.terminal()).count())
}

/// Name the source objects that finished their walk without appending a row.
///
/// The other half of §70 item 2's fact, and the half the owed count cannot carry: these objects
/// satisfy the item — a closed window is the object's own record that its walk finished — so they
/// travel as a retained finding instead. Without them the seal would show nothing at all for a source
/// of no rows, and a source that was read and found empty would read exactly like one nobody read.
/// Sorted and deduplicated, so the names are evidence rather than the order a caller named keys in.
pub fn silent_source_objects(objects: &[SourceObject]) -> Vec<String> {
    let mut names = objects
        .iter()
        .filter(|object| object.observations == 0 && object.windows > 0)
        .map(|object| object.endpoint.clone())
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    names
}

/// Count the retained cases that ask a cohort question and have no terminal decision.
///
/// The families are the domain's own list ([`COHORT_DECISION_FAMILIES`]) rather than a second copy
/// here: the mint rule decides which of their cases can start `Pending` at all, so a list this module
/// spelled itself could count a family the mint rule had already retired, or miss one it had not.
///
/// `Pending` is the state that says no decision was recorded, and `Resolved`, `Retained` and
/// `Superseded` are all terminal — the stored row stays visible in the workbook's queues whichever
/// they are. A case another lane owns is not a cohort decision and is counted by that lane's own item,
/// if it has one: the identity item below counts every family.
pub fn owed_cohort_decisions(cases: &[ReviewCase]) -> u64 {
    count(
        cases
            .iter()
            .filter(|case| case.state == ReviewState::Pending)
            .filter(|case| COHORT_DECISION_FAMILIES.contains(&case.family.as_str()))
            .count(),
    )
}

/// Count the retained cases no lane has decided, whatever family they belong to.
///
/// This is the identity item's own definition — every candidate the store retained has a terminal
/// deterministic or model-assisted decision — counted from the rows, because the row count is the
/// claim: the store's sequence counters report *appends*, and a table written wholesale appends once,
/// so a case count read from them says one while the table holds thousands.
pub fn owed_identity_candidates(cases: &[ReviewCase]) -> u64 {
    count(
        cases
            .iter()
            .filter(|case| case.state == ReviewState::Pending)
            .count(),
    )
}

/// A count that cannot be represented is not a count this census may claim.
fn count(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}
