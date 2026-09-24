//! The figures a seal records: the counts it certifies, the work that was still open when it
//! certified, the findings it retains, and the workbook it inspected.
//!
//! Types only. The rules that read them — which §70 item each figure decides — live in
//! [`super::SealEvidence`], beside the evidence they judge.

use serde::{Deserialize, Serialize};

/// What a seal certifies: the counts the census reads off the store and the coverage report.
///
/// Three of them are the report's counts for the rows it publishes under this run's scope rather than
/// totals over every stored row: `schools` and `cohort_performances` narrow by the census's cohort as
/// well, while `meets` is read whole under the scope.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealCounts {
    /// The rows the census's state rollup publishes: one per covered jurisdiction *and* the unplaced
    /// row the rollup always seeds, so this is not a count of placed jurisdictions.
    ///
    /// The field and the digest key were named `jurisdictions` until those two readings were told
    /// apart, and the digest prefix moved to `v4` with the rename so a `v3` digest cannot be read as
    /// this one. A `seal.json` written under the old name still reads.
    #[serde(alias = "jurisdictions")]
    pub jurisdiction_buckets: u64,
    pub schools: u64,
    pub meets: u64,
    pub athletes: u64,
    pub class_of_2027: u64,
    /// Performances counted from the published rows: the store's rows for athletes inside this census's
    /// cohort, under its run scope — not every performance the store holds. A reader who wants them all
    /// reads the workbook's `Performances_*` sheets or the store.
    ///
    /// The field and the digest key were named `performances` until that reading was told apart, and the
    /// digest prefix moved to `v5` with the rename so a `v4` digest cannot be read as this one. A
    /// `seal.json` written under the old name still reads.
    #[serde(alias = "performances")]
    pub cohort_performances: u64,
    pub coaches: u64,
}

/// Work that has not reached a terminal decision. Every field is a count that must be zero.
///
/// `None` means **unmeasured**, deliberately not `0`: a caller that does not read the workflow
/// journal — the store path of `census-service seal` is one — must say so, and `Default` is `None`
/// so a forgotten field refuses instead of sealing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenWork {
    pub jurisdiction_sweeps: Option<u64>,
    pub source_objects: Option<u64>,
    pub cohort_decisions: Option<u64>,
    pub identity_candidates: Option<u64>,
}

/// Findings a seal keeps rather than refuses: gaps, conflicts, the access conditions that stopped a
/// lane, source outages.
///
/// `source_failures` is `None` when the caller cannot count them, like [`OpenWork`]'s fields: a zero
/// that was never measured is a false claim, and this one lands in the digest.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetainedFindings {
    pub gaps: Vec<GapTally>,
    pub conflicts: u64,
    /// Source access conditions the store retained: one row per host per kind.
    ///
    /// This is the store's own count of the hosts that blocked or throttled a lane. It is not §70's
    /// *retry-exhausted operation*: exhaustion is an invocation state on the durable side, and no store
    /// row records it. The field carried that name until the two were told apart, and the digest prefix
    /// moved to `v3` with the rename so a `v2` digest cannot be read as this one.
    pub access_conditions: u64,
    /// Of [`Self::access_conditions`]: hosts the store recorded *refusing* this lane — `Forbidden`, or a
    /// robots denial the host did not authorize. Retrying one is an operator's decision rather than a
    /// cooldown's, which is why they are not lumped in with the throttles.
    pub blocked_hosts: u64,
    /// Of [`Self::access_conditions`]: hosts the store recorded asking the lane to slow down —
    /// `RateLimited`, `Timeout`, or a host that could not be reached. A cooldown bounds the retry; none
    /// of these is a refusal.
    pub throttled_hosts: u64,
    pub source_failures: Option<u64>,
    pub observations: u64,
    pub calculations: u64,
}

/// One gap class's count, retained in the seal so a reader can see what the census knows it lacks.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GapTally {
    pub class: String,
    pub unit: String,
    pub count: u64,
}

/// The workbook a recruiter opens, and the verdict that it reconciles with the store.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkbookCheck {
    pub sheets: u64,
    pub rows: u64,
    pub digests: Vec<String>,
    pub mapped_athletes: u64,
    pub counts_reconciled: bool,
    pub coverage_reconciled: bool,
    pub metrics_reconciled: bool,
    pub export_verified: bool,
    pub discrepancies: Vec<String>,
}
