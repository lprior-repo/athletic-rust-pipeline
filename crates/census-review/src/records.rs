use census_domain::model::{ReviewCase, ReviewState, ReviewVerdict, ReviewVerdictRecord};

use super::verdicts::Adjudication;

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

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReviewReport {
    pub requested: usize,
    pub accepted: usize,
    pub rejected: usize,
    pub insufficient: usize,
    pub dropped: usize,
    pub unanswered: usize,
    pub failed: usize,
    pub answered: usize,
}

impl ReviewReport {
    pub const fn resolved(&self) -> usize {
        self.accepted.saturating_add(self.insufficient)
    }

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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct CaseTally {
    pub(super) accepted: usize,
    pub(super) rejected: usize,
    pub(super) insufficient: usize,
    pub(super) answered: bool,
}

impl ReviewReport {
    pub(super) fn absorb(&mut self, tally: CaseTally) {
        self.accepted = self.accepted.saturating_add(tally.accepted);
        self.rejected = self.rejected.saturating_add(tally.rejected);
        self.insufficient = self.insufficient.saturating_add(tally.insufficient);
        if !tally.answered {
            self.unanswered = self.unanswered.saturating_add(1);
        }
    }
}

pub(super) fn record_case(
    case: &ReviewCase,
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
        closed_case.state = state_after(&adjudication);
        closed.push(closed_case);
    }
    (rows, closed, tally)
}

fn state_after(adjudication: &Adjudication) -> ReviewState {
    match adjudication {
        Adjudication::Decided(_) => ReviewState::Resolved,
        Adjudication::Undecided | Adjudication::Refused(_) => ReviewState::Retained,
    }
}
