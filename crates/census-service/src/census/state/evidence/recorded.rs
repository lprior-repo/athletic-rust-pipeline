
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealCounts {
    #[serde(alias = "jurisdictions")]
    pub jurisdiction_buckets: u64,
    pub schools: u64,
    pub meets: u64,
    pub athletes: u64,
    pub class_of_2027: u64,
    #[serde(alias = "performances")]
    pub cohort_performances: u64,
    pub coaches: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenWork {
    pub jurisdiction_sweeps: Option<u64>,
    pub source_objects: Option<u64>,
    pub cohort_decisions: Option<u64>,
    pub identity_candidates: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetainedFindings {
    pub gaps: Vec<GapTally>,
    pub conflicts: u64,
    pub access_conditions: u64,
    pub blocked_hosts: u64,
    pub throttled_hosts: u64,
    pub silent_sources: Vec<String>,
    pub source_failures: Option<u64>,
    pub observations: u64,
    pub calculations: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GapTally {
    pub class: String,
    pub unit: String,
    pub count: u64,
}

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
