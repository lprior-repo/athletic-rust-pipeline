use crate::public_identity_tests::support::TestResult;
use crate::{EvidenceSubjectAttachmentStoreError as AttachmentError, StoreError, StoreResult};
use census_domain::evidence_subject_attachment::{
    AttachmentDigestV1, AttachmentObjectReferenceV1, AttachmentReplayGrantV1,
    EvidenceSubjectAttachmentError,
};
use std::io::Write;

pub(super) const LIMIT: usize = AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS;

#[derive(Debug, Clone)]
pub(super) struct Refusal {
    pub label: &'static str,
    pub needed: usize,
}

#[derive(Default, Clone)]
pub(super) struct Meter {
    pub total: usize,
    pub admitted: usize,
    pub refusal: Option<Refusal>,
}

impl Meter {
    pub fn charge(&mut self, label: &'static str, amount: usize) -> TestResult {
        self.total = self
            .total
            .checked_add(amount)
            .ok_or("phase work overflow")?;
        if self.refusal.is_none() {
            if self.total > LIMIT {
                self.refusal = Some(Refusal {
                    label,
                    needed: self.total,
                });
            } else {
                self.admitted = self.total;
            }
        }
        Ok(())
    }

    pub fn singles(&mut self, label: &'static str, count: usize) -> TestResult {
        for _ in 0..count {
            self.charge(label, 1)?;
        }
        Ok(())
    }

    pub fn inline(&mut self) -> TestResult {
        self.charge("inline length", 1)?;
        self.charge("inline decode", 2)?;
        self.charge("inline hash", 1)
    }

    pub fn result<T>(
        &self,
        result: StoreResult<T>,
        operation: &str,
        phase: &str,
    ) -> TestResult<Option<T>> {
        match (&self.refusal, result) {
            (None, Ok(value)) => {
                self.report(operation, phase, "completed")?;
                Ok(Some(value))
            }
            (
                Some(expected),
                Err(StoreError::EvidenceSubjectAttachment(AttachmentError::Domain(
                    EvidenceSubjectAttachmentError::Capacity {
                        resource,
                        needed,
                        available,
                    },
                ))),
            ) => {
                check!(eq; resource, "aggregate traversal steps");
                check!(eq; needed, expected.needed);
                check!(eq; available, LIMIT);
                check!(!expected.label.is_empty());
                self.report(operation, phase, "capacity_refusal")?;
                Ok(None)
            }
            (expected, Err(error)) => {
                Err(format!("phase prediction {expected:?}, actual error {error:?}").into())
            }
            (Some(expected), Ok(_)) => {
                Err(format!("expected phase refusal {expected:?}, operation succeeded").into())
            }
        }
    }

    fn report(&self, operation: &str, phase: &str, outcome: &str) -> TestResult {
        let report = serde_json::json!({
            "control": "explicitly_synthetic_attachment_phase_work_v1",
            "operation": operation, "phase": phase, "outcome": outcome,
            "source_predicted_total": self.total, "source_predicted_admitted": self.admitted,
            "first_refusing_pass": self.refusal.as_ref().map(|refusal| refusal.label),
            "needed": self.refusal.as_ref().map(|refusal| refusal.needed), "limit": LIMIT,
        });
        writeln!(
            std::io::stderr().lock(),
            "{}",
            serde_json::to_string(&report)?
        )?;
        Ok(())
    }
}

pub(super) fn position<T: PartialEq>(values: &[T], value: &T) -> TestResult<usize> {
    values
        .iter()
        .position(|item| item == value)
        .and_then(|index| index.checked_add(1))
        .ok_or_else(|| "phase reference absent".into())
}

pub(super) fn depth(
    values: &[AttachmentObjectReferenceV1],
    digest: &AttachmentDigestV1,
) -> TestResult<usize> {
    let rank = values
        .iter()
        .take_while(|item| &item.digest < digest)
        .count();
    let present = values.get(rank).is_some_and(|item| &item.digest == digest);
    rank_depth(values.len(), rank, present)
}

fn rank_depth(mut length: usize, mut rank: usize, present: bool) -> TestResult<usize> {
    let mut depth = 0usize;
    for _ in 0..7 {
        if length == 0 {
            return Ok(depth);
        }
        depth = depth.checked_add(1).ok_or("depth overflow")?;
        let pivot = length / 2;
        if present && rank == pivot {
            return Ok(depth);
        }
        if rank <= pivot {
            length = pivot;
        } else {
            let offset = pivot.checked_add(1).ok_or("rank offset overflow")?;
            length = length.checked_sub(offset).ok_or("rank length underflow")?;
            rank = rank.checked_sub(offset).ok_or("rank underflow")?;
        }
    }
    if length == 0 {
        return Ok(depth);
    }
    Err("phase rank depth exceeds combined-reference bound".into())
}
