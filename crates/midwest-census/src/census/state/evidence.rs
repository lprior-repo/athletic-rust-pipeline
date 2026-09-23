//! What a census must prove about itself before §48 lets it complete.
//!
//! One value per acceptance item of §70, plus the numbers the seal certifies. Nothing here reads a
//! store or a report: the caller that holds those assembles the evidence, keeping these rules
//! testable on their own values.

use serde::{Deserialize, Serialize};

mod recorded;

pub use recorded::{GapTally, OpenWork, RetainedFindings, SealCounts, WorkbookCheck};

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
    /// Satisfied by *recording* the count, never by refusing over it, so it never appears in
    /// [`SealEvidence::open_items`]: a seal refused over a kept finding teaches operators to hide
    /// findings.
    ConflictsRetained,
    /// Every retry-exhausted operation is represented. What the store can represent of that is the
    /// access condition that stopped the lane, which [`RetainedFindings::access_conditions`] counts and
    /// splits; the exhaustion itself is an invocation state, not a stored row. Retained like
    /// [`Self::ConflictsRetained`].
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
    /// One open-work measurement, or the statement that the caller could not take it: a refusal has
    /// to name a number, and "not measured" is the honest answer when there is none.
    fn outstanding_of(count: Option<u64>, message: &str) -> String {
        match count {
            Some(count) => format!("{count} {message}"),
            None => {
                format!("{message}: not measured - this seal does not read the workflow journal")
            }
        }
    }

    /// The §70 items this evidence does not satisfy, in declaration order. Retained findings are
    /// deliberately absent: refusing a seal over a kept one would push an operator to hide it.
    pub fn open_items(&self) -> Vec<AcceptanceItem> {
        let mut open = Vec::new();
        // `Some(0)` is terminal, `Some(n)` is n outstanding, `None` is unmeasured — and unmeasured
        // stays open, so a missing measurement can never pass as a zero.
        if self.open.jurisdiction_sweeps != Some(0) {
            open.push(AcceptanceItem::JurisdictionSweepsTerminal);
        }
        if self.open.source_objects != Some(0) {
            open.push(AcceptanceItem::SourceObjectsTerminal);
        }
        if self.open.cohort_decisions != Some(0) {
            open.push(AcceptanceItem::CohortDecisionsTerminal);
        }
        if self.open.identity_candidates != Some(0) {
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
            AcceptanceItem::JurisdictionSweepsTerminal => Self::outstanding_of(
                self.open.jurisdiction_sweeps,
                "jurisdiction sweeps are not terminal",
            ),
            AcceptanceItem::SourceObjectsTerminal => Self::outstanding_of(
                self.open.source_objects,
                "source objects have no terminal state",
            ),
            AcceptanceItem::CohortDecisionsTerminal => Self::outstanding_of(
                self.open.cohort_decisions,
                "cohort decisions are unresolved",
            ),
            AcceptanceItem::IdentityCandidatesTerminal => Self::outstanding_of(
                self.open.identity_candidates,
                "identity candidates are undecided",
            ),
            AcceptanceItem::ConflictsRetained => {
                format!("{} retained conflicts", self.retained.conflicts)
            }
            AcceptanceItem::RetriesRepresented => format!(
                "{} source access conditions retained: {} hosts refused, {} throttled",
                self.retained.access_conditions,
                self.retained.blocked_hosts,
                self.retained.throttled_hosts
            ),
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
    /// Reached only for the five items `detail` sends here; every other item is measured above.
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
