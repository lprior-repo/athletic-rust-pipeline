//! The durable rows one pass writes, and the report it returns.
//!
//! A verdict is evidence, not an edit: a row records what the model answered and whether local
//! validation admitted it, while the canonical tables stay the merge's. Two records for one case
//! never merge — the stored row is the one an operator already read.
//!
//! # Report stages
//!
//! The report tracks:
//! - `requested`: cases selected for asking (this crate can observe)
//! - `answered`: answers returned, including failures and cancellations (this crate can observe)
//! - `decided`: real decisions via `Adjudication::Decided` (this crate can observe)
//! - `accepted`: verdict rows written to the verdict table (this crate can observe)
//! - `applied`: whether a verdict was applied to an athlete's identity (measured by the reporting layer)
//!
//! A deterministic decision is not a model call. A stored verdict count is not an accuracy
//! measurement.

use census_domain::model::{ReviewCase, ReviewState, ReviewVerdict, ReviewVerdictRecord};

use super::verdicts::Adjudication;
use super::ReviewFamily;

/// Build the durable row for one case's verdict.
fn verdict_record(
    case: &ReviewCase,
    verdict: &ReviewVerdict,
    adjudication: &Adjudication,
    reviewer: &str,
    observed_at: &str,
) -> ReviewVerdictRecord {
    let admitted = adjudication.admitted();
    let rationale = match adjudication {
        Adjudication::Refused(reason) => match reason.rationale() {
            Some(flag) => format!(
                "{} Refused `same_person`: packet flag `{flag}` is a hard contradiction.",
                verdict.rationale
            ),
            None => verdict.rationale.clone(),
        },
        Adjudication::Decided(_) | Adjudication::Undecided => verdict.rationale.clone(),
    };
    ReviewVerdictRecord {
        id: verdict.case_id.clone(),
        case_id: verdict.case_id.clone(),
        subject_id: case.subject_id.clone(),
        family: case.family.clone(),
        member_ids: case.member_ids.clone(),
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
        rationale,
        reviewer: reviewer.to_string(),
        observed_at: observed_at.to_string(),
    }
}

/// What one review pass did.
///
/// Tracks four stages this crate can observe: `requested` (cases selected), `answered` (answers
/// returned, including failures), `decided` (real decisions via `Adjudication::Decided`), and
/// `accepted` (verdict rows written). The fifth stage, `applied` — whether a verdict was applied
/// to an athlete's identity — is measured by the reporting layer, not this crate.
///
/// A deterministic decision is not a model call. A stored verdict count is not an accuracy
/// measurement.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReviewReport {
    /// Cases the pass selected for asking.
    pub requested: usize,
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
    /// Answers that came back (Answered variants).
    pub answered: usize,
}

impl ReviewReport {
    /// Cases this pass closed, either way.
    pub const fn resolved(&self) -> usize {
        self.accepted.saturating_add(self.insufficient)
    }

    /// One line for the CLI: what was asked, what came back, what was kept.
    pub fn summary(&self) -> String {
        format!(
            "requested={} answered={} decided={} accepted={} rejected={} insufficient={} unanswered={} dropped={} failed={}",
            self.requested,
            self.answered,
            self.accepted,
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
///
/// A decision resolves a case. An insufficient-evidence answer settles an athlete case as
/// retained for this evidence snapshot: it is terminal here so the lane does not repeatedly ask the
/// same unanswered question, while new evidence reopens the question by minting a new case id.
/// A refusal is a finding about the model, so it is retained as well, with the answer the model gave
/// rather than nothing.
pub(super) fn record_case(
    case: &ReviewCase,
    family: ReviewFamily,
    verdicts: Vec<(ReviewVerdict, Adjudication)>,
    reviewer: &str,
    observed_at: &str,
) -> (Vec<ReviewVerdictRecord>, Vec<ReviewCase>, CaseTally) {
    let mut rows = Vec::new();
    let mut closed = Vec::new();
    let mut tally = CaseTally::default();
    for (verdict, adjudication) in verdicts {
        tally.answered = true;
        match &adjudication {
            Adjudication::Decided(_) => tally.accepted = tally.accepted.saturating_add(1),
            Adjudication::Undecided => tally.insufficient = tally.insufficient.saturating_add(1),
            Adjudication::Refused(_) => tally.rejected = tally.rejected.saturating_add(1),
        }
        rows.push(verdict_record(
            case,
            &verdict,
            &adjudication,
            reviewer,
            observed_at,
        ));
        let mut closed_case = case.clone();
        closed_case.state = state_after(family, &adjudication);
        closed.push(closed_case);
    }
    (rows, closed, tally)
}

/// The state one answer moves its case to.
fn state_after(_family: ReviewFamily, adjudication: &Adjudication) -> ReviewState {
    match adjudication {
        Adjudication::Decided(_) => ReviewState::Resolved,
        Adjudication::Undecided | Adjudication::Refused(_) => ReviewState::Retained,
    }
}
