//! The durable rows one pass writes, and the report it returns.
//!
//! A verdict is evidence, not an edit: a row records what the model answered and whether local
//! validation admitted it, while the canonical tables stay the merge's. Two records for one case
//! never merge — the stored row is the one an operator already read.

use census_domain::model::{
    ReviewCase, ReviewState, ReviewVerdict, ReviewVerdictKind, ReviewVerdictRecord,
};

use crate::store::Entity;

use super::verdicts::Admitted;

/// A verdict is evidence, so two records for one case never merge: the stored row is the one an
/// operator already read.
impl Entity for ReviewVerdictRecord {
    fn entity_id(&self) -> &str {
        &self.id
    }

    fn merge(&mut self, _other: Self) {}
}

/// Build the durable row for one case's verdict.
fn verdict_record(
    subject_id: &str,
    family: &str,
    verdict: &ReviewVerdict,
    admitted: Option<&Admitted>,
    reviewer: &str,
    observed_at: &str,
) -> ReviewVerdictRecord {
    ReviewVerdictRecord {
        id: verdict.case_id.clone(),
        case_id: verdict.case_id.clone(),
        subject_id: subject_id.to_string(),
        family: family.to_string(),
        kind: verdict.kind.slug().to_string(),
        field: admitted
            .map(|admitted| admitted.field.clone())
            .or_else(|| verdict.field.clone())
            .unwrap_or_default(),
        value: admitted
            .map(|admitted| admitted.value.clone())
            .or_else(|| verdict.value.clone())
            .unwrap_or_default(),
        accepted: admitted.is_some(),
        confidence: verdict.confidence,
        rationale: verdict.rationale.clone(),
        reviewer: reviewer.to_string(),
        observed_at: observed_at.to_string(),
    }
}

/// What one review pass did.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReviewReport {
    /// Cases the pass selected.
    pub asked: usize,
    /// Cases whose proposal validation admitted.
    pub accepted: usize,
    /// Cases whose proposal validation refused.
    pub rejected: usize,
    /// Cases the model declined to decide.
    pub insufficient: usize,
    /// Verdicts dropped because they answered a case that was not asked, or answered one twice.
    pub dropped: usize,
    /// Asked cases no verdict came back for.
    pub unanswered: usize,
    /// Requests that failed outright.
    pub failed: usize,
}

impl ReviewReport {
    /// Cases this pass closed, either way.
    pub const fn resolved(&self) -> usize {
        self.accepted.saturating_add(self.insufficient)
    }

    /// One line for the CLI: what was asked, what came back, what was kept.
    pub fn summary(&self) -> String {
        format!(
            "asked={} accepted={} rejected={} insufficient={} unanswered={} dropped={} failed={}",
            self.asked,
            self.accepted,
            self.rejected,
            self.insufficient,
            self.unanswered,
            self.dropped,
            self.failed
        )
    }
}

/// What one asked case's verdicts add to a pass report.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct CaseTally {
    pub(super) accepted: usize,
    pub(super) rejected: usize,
    pub(super) insufficient: usize,
    /// Whether any verdict answered the case at all.
    pub(super) answered: bool,
}

impl ReviewReport {
    /// Fold one case's tally into the report: the verdict kinds it closed, and, when nothing
    /// answered the case at all, the case as unanswered.
    pub(super) fn absorb(&mut self, tally: CaseTally) {
        self.accepted = self.accepted.saturating_add(tally.accepted);
        self.rejected = self.rejected.saturating_add(tally.rejected);
        self.insufficient = self.insufficient.saturating_add(tally.insufficient);
        if !tally.answered {
            self.unanswered = self.unanswered.saturating_add(1);
        }
    }
}

/// Build one asked case's durable rows, the states its case moves to, and the tally they contribute.
///
/// The caller owns the tables: this reads only what the case and its verdicts say.
pub(super) fn record_case(
    case: &ReviewCase,
    verdicts: Vec<(ReviewVerdict, Option<Admitted>)>,
    reviewer: &str,
    observed_at: &str,
) -> (Vec<ReviewVerdictRecord>, Vec<ReviewCase>, CaseTally) {
    let mut rows = Vec::new();
    let mut closed = Vec::new();
    let mut tally = CaseTally::default();
    for (verdict, admitted) in verdicts {
        tally.answered = true;
        match verdict.kind {
            ReviewVerdictKind::ValueProposed if admitted.is_some() => {
                tally.accepted = tally.accepted.saturating_add(1);
            }
            ReviewVerdictKind::ValueProposed => {
                tally.rejected = tally.rejected.saturating_add(1);
            }
            ReviewVerdictKind::InsufficientEvidence => {
                tally.insufficient = tally.insufficient.saturating_add(1);
            }
        }
        rows.push(verdict_record(
            &case.subject_id,
            &case.family,
            &verdict,
            admitted.as_ref(),
            reviewer,
            observed_at,
        ));
        let mut closed_case = case.clone();
        closed_case.state = if admitted.is_some() {
            ReviewState::Resolved
        } else {
            ReviewState::Retained
        };
        closed.push(closed_case);
    }
    (rows, closed, tally)
}
