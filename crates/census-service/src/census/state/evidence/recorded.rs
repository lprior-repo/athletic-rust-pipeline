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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GapDigestInputs {
    pub gaps: Vec<GapTally>,
    pub silent_sources: Vec<String>,
    pub source_failures: Option<u64>,
}

impl RetainedFindings {
    pub fn gap_digest_inputs(&self) -> GapDigestInputs {
        let mut gaps = self.gaps.clone();
        gaps.sort();
        let mut silent_sources = self.silent_sources.clone();
        silent_sources.sort();
        silent_sources.dedup();
        GapDigestInputs {
            gaps,
            silent_sources,
            source_failures: self.source_failures,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verification {
    Verified,
    Failed,
    #[default]
    NotMeasured,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkbookCheck {
    pub sheets: u64,
    pub rows: u64,
    pub digests: Vec<String>,
    pub mapped_athletes: u64,
    pub counts_reconciled: Verification,
    pub coverage_reconciled: Verification,
    pub metrics_reconciled: Verification,
    pub export_verified: Verification,
    pub discrepancies: Vec<String>,
}
