use census_domain::model::{ReviewCase, ReviewState, ReviewVerdictRecord};
use census_store::StoreResult;

use super::ask::Answer;
use super::consensus::{invariant, Asked, Audit, Consensus, POLICY};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReviewReport {
    pub requested: usize,
    pub unaskable: usize,
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
        self.accepted
    }

    pub fn summary(&self) -> String {
        format!(
            "requested={} unaskable={} answered={} decided={} accepted={} rejected={} insufficient={} unanswered={} dropped={} failed={}",
            self.requested, self.unaskable, self.answered, self.accepted, self.accepted,
            self.rejected, self.insufficient, self.unanswered, self.dropped, self.failed
        )
    }

    fn account(&mut self, asked: &Asked, consensus: &Consensus) {
        if asked.packet.is_none() {
            self.unaskable = self.unaskable.saturating_add(1);
            return;
        }
        let Some(answers) = asked.answers.as_ref() else {
            self.unanswered = self.unanswered.saturating_add(1);
            return;
        };
        let answered = answers.iter().all(|answer| {
            matches!(answer,
            Answer::Answered { verdicts, .. } if !verdicts.is_empty())
        });
        if answered {
            self.answered = self.answered.saturating_add(1);
        } else {
            self.unanswered = self.unanswered.saturating_add(1);
        }
        answers.iter().for_each(|answer| match answer {
            Answer::Failed(_) => self.failed = self.failed.saturating_add(1),
            Answer::Answered { dropped, .. } => {
                self.dropped = self.dropped.saturating_add(*dropped)
            }
        });
        if consensus.admitted.is_some() {
            self.accepted = self.accepted.saturating_add(1);
        } else if consensus.rejected {
            self.rejected = self.rejected.saturating_add(1);
        } else if answered {
            self.insufficient = self.insufficient.saturating_add(1);
        }
    }
}

pub(super) fn confidence(asked: &Asked) -> u8 {
    asked
        .answers
        .as_ref()
        .and_then(|answers| {
            answers
                .iter()
                .filter_map(|answer| match answer {
                    Answer::Answered { verdicts, .. } => {
                        verdicts.first().map(|(verdict, _)| verdict.confidence)
                    }
                    Answer::Failed(_) => None,
                })
                .min()
        })
        .map_or(0, |confidence| confidence)
}

pub(super) fn record_case(
    case: &ReviewCase,
    asked: Asked,
    consensus: Consensus,
    observed_at: &str,
    report: &mut ReviewReport,
) -> StoreResult<(ReviewVerdictRecord, ReviewCase)> {
    report.account(&asked, &consensus);
    let accepted = consensus.admitted.is_some();
    let confidence = if accepted { confidence(&asked) } else { 0 };
    let (field, value) = consensus.admitted.map_or_else(
        || (String::new(), String::new()),
        |admitted| (admitted.field, admitted.value),
    );
    let evidence_digest = census_domain::model::serialized_digest(&asked.packet)
        .map_err(|error| invariant(format!("cannot bind review evidence: {error}")))?;
    let [first, second] = asked.lanes;
    let lanes = match asked.answers {
        Some([first_answer, second_answer]) => {
            [first.retain(first_answer), second.retain(second_answer)]
        }
        None => [first, second],
    };
    let audit = Audit {
        policy: POLICY.to_string(),
        evidence_digest,
        packet: asked.packet,
        lanes,
        outcome: consensus.reason.to_string(),
    };
    let rationale = audit.encode()?;
    let row = ReviewVerdictRecord {
        id: case.id.clone(),
        case_id: case.id.clone(),
        subject_id: case.subject_id.clone(),
        family: case.family.clone(),
        member_ids: case.member_ids.clone(),
        kind: if accepted {
            "value_proposed"
        } else {
            "insufficient_evidence"
        }
        .to_string(),
        field,
        value,
        accepted,
        confidence,
        rationale,
        reviewer: "dual-independent-consensus".to_string(),
        observed_at: observed_at.to_string(),
    };
    let mut state = case.clone();
    state.state = if accepted {
        ReviewState::Resolved
    } else {
        ReviewState::Retained
    };
    Ok((row, state))
}
