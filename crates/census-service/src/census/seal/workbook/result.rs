use crate::census::WorkbookCheck;

use super::count;

/// The reconciled values the caller assembled, ready to become a [`WorkbookCheck`].
pub(super) struct CheckAssembly<'a> {
    pub(super) names: &'a [&'a str],
    pub(super) coverage_unique_count: usize,
    pub(super) run_metrics: &'a [Vec<String>],
    pub(super) digest: String,
    pub(super) mapped_athletes: u64,
    pub(super) decisions: CheckDecisions,
    pub(super) discrepancies: Vec<String>,
}

/// The four independent verdicts the check carries, kept together so the call stays readable.
pub(super) struct CheckDecisions {
    pub(super) counts_reconciled: bool,
    pub(super) coverage_reconciled: bool,
    pub(super) metrics_reconciled: bool,
    pub(super) export_verified: bool,
}

/// Assemble the final [`WorkbookCheck`] from intermediate reconciliation values.
pub(super) fn build_check(parts: CheckAssembly<'_>) -> WorkbookCheck {
    WorkbookCheck {
        sheets: count(parts.names.len()),
        rows: count(
            parts
                .coverage_unique_count
                .saturating_add(parts.run_metrics.len().saturating_sub(1)),
        ),
        digests: vec![parts.digest],
        mapped_athletes: parts.mapped_athletes,
        counts_reconciled: parts.decisions.counts_reconciled,
        coverage_reconciled: parts.decisions.coverage_reconciled,
        metrics_reconciled: parts.decisions.metrics_reconciled,
        export_verified: parts.decisions.export_verified,
        discrepancies: parts.discrepancies,
    }
}
