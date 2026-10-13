use super::JurisdictionCoverageState;
use census_domain::model::{CoachLaneMailbox, CoachLaneObligation, CoachLaneStatus};
use census_domain::{JurisdictionBucket, UsJurisdiction};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ContactCounts {
    pub attempted: usize,
    pub reached: usize,
    pub failed: usize,
    pub blocked: usize,
    pub never_attempted: usize,
    pub accepted_refusal: usize,
    pub exhausted: usize,
    pub successful_empty: usize,
    pub complete_with_gaps: usize,
    pub pending: usize,
}

impl ContactCounts {
    pub(super) fn observe(&mut self, lane: &CoachLaneObligation) {
        let status = lane.status();
        if status != CoachLaneStatus::Pending {
            self.attempted = self.attempted.saturating_add(1);
        }
        let bump = match status {
            CoachLaneStatus::Pending => &mut self.pending,
            CoachLaneStatus::Blocked => &mut self.blocked,
            CoachLaneStatus::Failed => &mut self.failed,
            CoachLaneStatus::AcceptedRefusal => &mut self.accepted_refusal,
            CoachLaneStatus::Exhausted => &mut self.exhausted,
            CoachLaneStatus::SuccessfulEmpty => &mut self.successful_empty,
            CoachLaneStatus::CompleteWithGaps => &mut self.complete_with_gaps,
            CoachLaneStatus::Armed => &mut self.attempted,
        };
        if status != CoachLaneStatus::Armed {
            *bump = bump.saturating_add(1);
        }
        if lane.resolution().mailbox == CoachLaneMailbox::Published {
            self.reached = self.reached.saturating_add(1);
        }
    }

    pub(super) fn merge(&self, other: &Self) -> Self {
        Self {
            attempted: self.attempted.saturating_add(other.attempted),
            reached: self.reached.saturating_add(other.reached),
            failed: self.failed.saturating_add(other.failed),
            blocked: self.blocked.saturating_add(other.blocked),
            never_attempted: self.never_attempted.saturating_add(other.never_attempted),
            accepted_refusal: self.accepted_refusal.saturating_add(other.accepted_refusal),
            exhausted: self.exhausted.saturating_add(other.exhausted),
            successful_empty: self.successful_empty.saturating_add(other.successful_empty),
            complete_with_gaps: self
                .complete_with_gaps
                .saturating_add(other.complete_with_gaps),
            pending: self.pending.saturating_add(other.pending),
        }
    }

    pub(super) fn terminal(&self) -> usize {
        self.blocked
            .saturating_add(self.failed)
            .saturating_add(self.accepted_refusal)
            .saturating_add(self.exhausted)
            .saturating_add(self.successful_empty)
            .saturating_add(self.complete_with_gaps)
    }
}

#[derive(Debug, Default)]
pub(super) struct ContactPlane {
    counts: BTreeMap<JurisdictionBucket, ContactCounts>,
}

impl ContactPlane {
    pub(super) fn of<'a>(lanes: impl IntoIterator<Item = &'a CoachLaneObligation>) -> Self {
        let mut counts: BTreeMap<JurisdictionBucket, ContactCounts> = BTreeMap::new();
        for lane in lanes {
            let bucket = JurisdictionBucket::from(lane.jurisdiction());
            counts.entry(bucket).or_default().observe(lane);
        }
        Self { counts }
    }

    pub(super) fn for_bucket(&self, bucket: JurisdictionBucket) -> ContactCounts {
        match self.counts.get(&bucket) {
            Some(counts) => *counts,
            None => ContactCounts::default(),
        }
    }

    pub(super) fn declare_scope(&mut self, scoped: impl IntoIterator<Item = UsJurisdiction>) {
        for jurisdiction in scoped {
            let bucket = JurisdictionBucket::from(jurisdiction);
            let counts = self.counts.entry(bucket).or_default();
            if counts.attempted == 0 && counts.pending == 0 {
                counts.never_attempted = 1;
            }
        }
    }

    pub(super) fn state(&self, bucket: JurisdictionBucket) -> JurisdictionCoverageState {
        let counts = self.for_bucket(bucket);
        if counts.attempted == 0 && counts.pending == 0 {
            return JurisdictionCoverageState::Unresearched;
        }
        if counts.pending > 0 {
            return JurisdictionCoverageState::Pending;
        }
        if counts.blocked > 0 {
            return JurisdictionCoverageState::Blocked;
        }
        if counts.failed > 0 || counts.exhausted > 0 {
            return JurisdictionCoverageState::RetainedGap;
        }
        JurisdictionCoverageState::Complete
    }

    pub(super) fn totals(&self) -> ContactCounts {
        self.counts
            .values()
            .copied()
            .fold(ContactCounts::default(), |mut merged, next| {
                merged = merged.merge(&next);
                merged
            })
    }
}
