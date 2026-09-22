//! What a census must prove about itself before §48 lets it complete.
//!
//! One value per acceptance item of §70, plus the numbers the seal certifies. Nothing here reads a
//! store or a report: the evidence is assembled by the caller that holds those, so the acceptance
//! rules stay testable on their own values.

use serde::{Deserialize, Serialize};

/// The acceptance items of §70, in the order that list states them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcceptanceItem {
    /// Every configured jurisdiction has terminal source sweeps.
    JurisdictionSweepsTerminal,
    /// Every discovered source object has a terminal acquisition state.
    SourceObjectsTerminal,
    /// Every potential cohort athlete has a terminal cohort decision.
    CohortDecisionsTerminal,
    /// Every identity candidate has a terminal deterministic or model-assisted decision.
    IdentityCandidatesTerminal,
    /// Every unresolved conflict is retained.
    ///
    /// Satisfied by the seal *recording* the count, not by refusing over it, so it never appears in
    /// [`SealEvidence::open_items`]: §70 asks that conflicts be kept, and a seal refused over a kept
    /// finding would push an operator to hide one.
    ConflictsRetained,
    /// Every retry-exhausted operation is represented. Retained like [`Self::ConflictsRetained`].
    RetriesRepresented,
    /// Successful evidence is durable.
    EvidenceDurable,
    /// Performance and PR calculations are reproducible.
    CalculationsReproducible,
    /// Every workbook athlete maps to canonical stored evidence.
    WorkbookMapped,
    /// Workbook counts reconcile against the store.
    WorkbookCountsReconcile,
    /// The source coverage report reconciles.
    CoverageReportReconciles,
    /// Run metrics reconcile.
    RunMetricsReconcile,
    /// Final export verification passes.
    ExportVerified,
}

impl AcceptanceItem {
    /// Every item, in declaration order — the list a status command prints.
    pub const ALL: [AcceptanceItem; 13] = [
        AcceptanceItem::JurisdictionSweepsTerminal,
        AcceptanceItem::SourceObjectsTerminal,
        AcceptanceItem::CohortDecisionsTerminal,
        AcceptanceItem::IdentityCandidatesTerminal,
        AcceptanceItem::ConflictsRetained,
        AcceptanceItem::RetriesRepresented,
        AcceptanceItem::EvidenceDurable,
        AcceptanceItem::CalculationsReproducible,
        AcceptanceItem::WorkbookMapped,
        AcceptanceItem::WorkbookCountsReconcile,
        AcceptanceItem::CoverageReportReconciles,
        AcceptanceItem::RunMetricsReconcile,
        AcceptanceItem::ExportVerified,
    ];

    /// The name a refusal prints, matching §70's wording.
    pub const fn as_str(self) -> &'static str {
        match self {
            AcceptanceItem::JurisdictionSweepsTerminal => "jurisdiction sweeps are terminal",
            AcceptanceItem::SourceObjectsTerminal => "source objects are terminal",
            AcceptanceItem::CohortDecisionsTerminal => "cohort decisions are terminal",
            AcceptanceItem::IdentityCandidatesTerminal => "identity candidates are terminal",
            AcceptanceItem::ConflictsRetained => "unresolved conflicts are retained",
            AcceptanceItem::RetriesRepresented => "retry-exhausted operations are represented",
            AcceptanceItem::EvidenceDurable => "successful evidence is durable",
            AcceptanceItem::CalculationsReproducible => "PR calculations are reproducible",
            AcceptanceItem::WorkbookMapped => "workbook athletes map to canonical evidence",
            AcceptanceItem::WorkbookCountsReconcile => "workbook counts reconcile with the store",
            AcceptanceItem::CoverageReportReconciles => "the coverage report reconciles",
            AcceptanceItem::RunMetricsReconcile => "run metrics reconcile",
            AcceptanceItem::ExportVerified => "final export verification passes",
        }
    }
}

/// What a seal certifies: the numbers a reader of the workbook sees.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealCounts {
    pub jurisdictions: u64,
    pub schools: u64,
    pub meets: u64,
    pub athletes: u64,
    pub class_of_2027: u64,
    pub performances: u64,
    pub coaches: u64,
}

/// Work that has not reached a terminal decision. Every field is a count that must be zero.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenWork {
    pub jurisdiction_sweeps: u64,
    pub source_objects: u64,
    pub cohort_decisions: u64,
    pub identity_candidates: u64,
}

/// Findings a seal keeps rather than refuses: gaps, conflicts, exhausted retries, source outages.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetainedFindings {
    pub gaps: Vec<GapTally>,
    pub conflicts: u64,
    pub retry_exhausted: u64,
    pub source_failures: u64,
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

/// Everything §70 asks a census to prove about itself before it may be sealed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealEvidence {
    pub open: OpenWork,
    pub counts: SealCounts,
    pub retained: RetainedFindings,
    pub workbook: WorkbookCheck,
    pub observed_on: String,
}

impl SealEvidence {
    /// The §70 items this evidence does not satisfy, in declaration order.
    ///
    /// Retained findings are deliberately absent from the result: a gap tally, a conflict count or
    /// an exhausted retry is a *finding* §70 wants kept, and refusing a seal over one would push an
    /// operator to hide it. The two items that appear here conditionally —
    /// [`AcceptanceItem::EvidenceDurable`] and [`AcceptanceItem::CalculationsReproducible`] — demand
    /// a non-zero measured count, because a census that claims athletes but holds no observations
    /// or no reproducible calculation has proved nothing.
    pub fn open_items(&self) -> Vec<AcceptanceItem> {
        let mut open = Vec::new();
        if self.open.jurisdiction_sweeps > 0 {
            open.push(AcceptanceItem::JurisdictionSweepsTerminal);
        }
        if self.open.source_objects > 0 {
            open.push(AcceptanceItem::SourceObjectsTerminal);
        }
        if self.open.cohort_decisions > 0 {
            open.push(AcceptanceItem::CohortDecisionsTerminal);
        }
        if self.open.identity_candidates > 0 {
            open.push(AcceptanceItem::IdentityCandidatesTerminal);
        }
        if self.retained.observations == 0 && self.counts.athletes > 0 {
            open.push(AcceptanceItem::EvidenceDurable);
        }
        if self.retained.calculations == 0 && self.counts.performances > 0 {
            open.push(AcceptanceItem::CalculationsReproducible);
        }
        if self.workbook.mapped_athletes < self.counts.class_of_2027 {
            open.push(AcceptanceItem::WorkbookMapped);
        }
        if !self.workbook.counts_reconciled {
            open.push(AcceptanceItem::WorkbookCountsReconcile);
        }
        if !self.workbook.coverage_reconciled {
            open.push(AcceptanceItem::CoverageReportReconciles);
        }
        if !self.workbook.metrics_reconciled {
            open.push(AcceptanceItem::RunMetricsReconcile);
        }
        if !self.workbook.export_verified || !self.workbook.discrepancies.is_empty() {
            open.push(AcceptanceItem::ExportVerified);
        }
        open
    }

    /// The measurement behind one item, so a refusal names a number rather than a category.
    pub fn detail(&self, item: AcceptanceItem) -> String {
        match item {
            AcceptanceItem::WorkbookMapped
            | AcceptanceItem::WorkbookCountsReconcile
            | AcceptanceItem::CoverageReportReconciles
            | AcceptanceItem::RunMetricsReconcile
            | AcceptanceItem::ExportVerified => self.workbook_detail(item),
            AcceptanceItem::JurisdictionSweepsTerminal => {
                format!(
                    "{} jurisdiction sweeps are not terminal",
                    self.open.jurisdiction_sweeps
                )
            }
            AcceptanceItem::SourceObjectsTerminal => {
                format!(
                    "{} source objects have no terminal state",
                    self.open.source_objects
                )
            }
            AcceptanceItem::CohortDecisionsTerminal => {
                format!(
                    "{} cohort decisions are unresolved",
                    self.open.cohort_decisions
                )
            }
            AcceptanceItem::IdentityCandidatesTerminal => {
                format!(
                    "{} identity candidates are undecided",
                    self.open.identity_candidates
                )
            }
            AcceptanceItem::ConflictsRetained => {
                format!("{} retained conflicts", self.retained.conflicts)
            }
            AcceptanceItem::RetriesRepresented => {
                format!(
                    "{} retry-exhausted operations",
                    self.retained.retry_exhausted
                )
            }
            AcceptanceItem::EvidenceDurable => format!(
                "{} durable observations for {} athletes",
                self.retained.observations, self.counts.athletes
            ),
            AcceptanceItem::CalculationsReproducible => format!(
                "{} reproducible calculations for {} performances",
                self.retained.calculations, self.counts.performances
            ),
        }
    }

    /// The measurements the workbook check supplies: what the exported bytes did or did not show.
    ///
    /// Reached only for the five items `detail` sends here; every other item is measured from the
    /// seal's own counts above.
    fn workbook_detail(&self, item: AcceptanceItem) -> String {
        match item {
            AcceptanceItem::WorkbookMapped => format!(
                "{} of {} cohort athletes appear in the workbook",
                self.workbook.mapped_athletes, self.counts.class_of_2027
            ),
            AcceptanceItem::WorkbookCountsReconcile => format!(
                "the workbook's meta sheets do not reconcile with the store: it names {} of {} cohort athletes",
                self.workbook.mapped_athletes, self.counts.class_of_2027
            ),
            AcceptanceItem::CoverageReportReconciles => {
                "the coverage report does not reconcile".to_string()
            }
            AcceptanceItem::RunMetricsReconcile => "the run metrics do not reconcile".to_string(),
            AcceptanceItem::ExportVerified => format!(
                "{} export discrepancies: {}",
                self.workbook.discrepancies.len(),
                self.workbook.discrepancies.join("; ")
            ),
            _ => "no workbook measurement for this item".to_string(),
        }
    }

    /// The seal this evidence grants. Callers reach it through `CensusState::seal`.
    pub(super) fn into_seal(self) -> SealedCensus {
        let digest = super::seal_digest::render(&self);
        SealedCensus {
            counts: self.counts,
            retained: self.retained,
            workbook_rows: self.workbook.rows,
            sealed_on: self.observed_on,
            digest,
        }
    }
}

/// A sealed census: the counts it certifies, the findings it keeps, and its digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealedCensus {
    pub counts: SealCounts,
    pub retained: RetainedFindings,
    pub workbook_rows: u64,
    pub sealed_on: String,
    pub digest: String,
}
