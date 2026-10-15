use crate::model::{Revision, SchoolYear};
use crate::UsJurisdiction;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionSummary {
    pub jurisdiction: UsJurisdiction,
    pub identity: String,
    pub rosters_total: usize,
    pub rosters_committed: usize,
    pub rosters_remaining: usize,
    pub rosters_skipped: usize,
    pub blocked: bool,
    pub athletes: usize,
    pub class_of_2027: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NationalFailure {
    pub jurisdiction: UsJurisdiction,
    pub identity: String,
    pub error: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NationalCompleteness {
    Partial,
    #[default]
    Final,
}

impl NationalCompleteness {
    pub fn is_partial(self) -> bool {
        matches!(self, Self::Partial)
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Partial => "partial",
            Self::Final => "final",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionOwed {
    pub identity: String,
    pub jurisdiction: UsJurisdiction,
    #[serde(default)]
    pub stages_run: Vec<String>,
    #[serde(default)]
    pub owed: Vec<String>,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Counters {
    pub scanned: u64,
    pub linked: u64,
    pub already_linked: u64,
    #[serde(default)]
    pub backfilled: u64,
    #[serde(default)]
    pub postal_gaps: u64,
    #[serde(default)]
    pub postal_filled: u64,
    #[serde(default)]
    pub postal_remaining: u64,
    #[serde(default)]
    pub ccd_filled: u64,
    #[serde(default)]
    pub pss_filled: u64,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchoolAddressJoinReply {
    pub mode: String,
    pub generation: String,
    pub report: String,
    pub outcomes: String,
    pub counters: Counters,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NationalReport {
    pub season: SchoolYear,
    pub revision: Revision,
    pub jurisdictions: Vec<JurisdictionSummary>,
    pub failures: Vec<NationalFailure>,
    #[serde(default)]
    pub owed: Vec<JurisdictionOwed>,
    pub rosters_total: usize,
    pub athletes_total: usize,
    pub class_of_2027_total: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub school_address: Option<SchoolAddressJoinReply>,
    pub today: String,
    #[serde(default)]
    pub completeness: NationalCompleteness,
}

impl NationalReport {
    pub fn is_partial(&self) -> bool {
        self.completeness.is_partial()
    }
    pub fn completeness_label(&self) -> &'static str {
        self.completeness.label()
    }
}
