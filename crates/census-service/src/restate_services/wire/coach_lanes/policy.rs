use census_domain::model::{
    CoachLaneError, CoachLaneEvidence, CoachLaneMailbox, CoachLaneObligation, CoachLaneRefusal,
    CoachLaneResolution, CoachLaneRole, CoachLaneStage, CoachLaneTenure, PendingCoachLane,
    SchoolYear, SourceRef, COACH_LANE_MAX_FRESHNESS_DAYS,
};
use census_domain::UsJurisdiction;

use super::freshness_days;
use crate::restate_services::plan::RefusalKind;

pub(super) struct Lane {
    pub(super) jurisdiction: UsJurisdiction,
    pub(super) arm: &'static str,
    pub(super) refusal: (&'static str, &'static str),
    pub(super) kind: RefusalKind,
    pub(super) basis: ArmBasis,
    pub(super) policy: LanePolicy,
}

pub(super) enum ArmBasis {
    Text(&'static str),
    MissingReason,
    Capture {
        text: &'static str,
        source_url: Option<&'static str>,
        path: &'static str,
        digest: &'static str,
    },
}

pub(super) enum LanePolicy {
    Blocked,
    Missing,
    RetainedCapture(&'static RetainedCapture),
    HeldCapture(&'static RetainedCapture),
}

pub(super) enum Acquisition<'a> {
    Available,
    Held(&'a str),
}

pub(super) struct RetainedCapture {
    pub(super) schools: u32,
    pub(super) contacts: u32,
    pub(super) digest: &'static str,
    pub(super) captured_on: &'static str,
    pub(super) source_url: &'static str,
    pub(super) mailbox: CoachLaneMailbox,
    pub(super) note: &'static str,
    pub(super) captures: &'static [(&'static str, &'static str)],
    pub(super) stale_path: &'static str,
    pub(super) jurisdiction_name: &'static str,
}

impl Lane {
    pub(super) fn evaluate(
        &self,
        season: SchoolYear,
        revision: u32,
        observed_on: &str,
        acquisition: Acquisition<'_>,
    ) -> Result<CoachLaneObligation, CoachLaneError> {
        let prior_refusal = match self.policy {
            LanePolicy::Missing => Some(self.refusal(observed_on)?),
            _ => None,
        };
        let pending = CoachLaneObligation::pending(PendingCoachLane {
            jurisdiction: self.jurisdiction,
            season,
            revision,
            arm: self.arm.to_string(),
            arm_basis: self.arm_basis(),
            observed_on: observed_on.to_string(),
        })?;
        match self.policy {
            LanePolicy::Missing => pending.record(
                CoachLaneStage::Blocked,
                prior_refusal,
                CoachLaneResolution::unresolved(observed_on),
            ),
            LanePolicy::Blocked => blocked(pending, self.refusal(observed_on)?, observed_on),
            LanePolicy::RetainedCapture(capture) => capture.resolve(pending, observed_on),
            LanePolicy::HeldCapture(capture) => match acquisition {
                Acquisition::Available => capture.resolve(pending, observed_on),
                Acquisition::Held(reason) => blocked(
                    pending,
                    CoachLaneRefusal::new(SourceRef::id(self.arm), reason, observed_on)?,
                    observed_on,
                ),
            },
        }
    }

    pub(super) fn refusal(&self, observed_on: &str) -> Result<CoachLaneRefusal, CoachLaneError> {
        CoachLaneRefusal::new(SourceRef::id(self.refusal.0), self.refusal.1, observed_on)
    }

    fn arm_basis(&self) -> String {
        match self.basis {
            ArmBasis::Text(text) => text.to_string(),
            ArmBasis::MissingReason => self.refusal.1.to_string(),
            ArmBasis::Capture {
                text,
                source_url,
                path,
                digest,
            } => match source_url {
                Some(url) => format!("{text} {url} {path} sha256 {digest}"),
                None => format!("{text} {path} sha256 {digest}"),
            },
        }
    }
}

impl RetainedCapture {
    fn resolve(
        &self,
        pending: CoachLaneObligation,
        observed_on: &str,
    ) -> Result<CoachLaneObligation, CoachLaneError> {
        let freshness = freshness_days(self.captured_on, observed_on)?;
        if freshness > COACH_LANE_MAX_FRESHNESS_DAYS {
            let refusal = CoachLaneRefusal::new(
                SourceRef::id(pending.arm_name()),
                format!(
                    "the held {} capture {} is {freshness} days old at lane creation, beyond the {COACH_LANE_MAX_FRESHNESS_DAYS}-day bound, so the {} arm cannot retain it",
                    pending.arm_name(), self.stale_path, self.jurisdiction_name,
                ),
                observed_on,
            )?;
            return blocked(pending, refusal, observed_on);
        }
        let evidence =
            CoachLaneEvidence::new(self.schools, self.contacts, self.digest, self.captured_on)?
                .with_source_url(self.source_url)?;
        let resolution = CoachLaneResolution::resolved(
            CoachLaneRole::Published,
            CoachLaneTenure::Current,
            self.mailbox,
            self.captured_on,
            freshness,
            self.resolved_note(),
        )?;
        pending.arm(evidence, resolution)
    }

    pub(super) fn resolved_note(&self) -> String {
        self.captures
            .iter()
            .fold(self.note.to_string(), |mut note, (path, digest)| {
                note.push_str(&format!(" {path} sha256 {digest};"));
                note
            })
    }
}

fn blocked(
    pending: CoachLaneObligation,
    refusal: CoachLaneRefusal,
    observed_on: &str,
) -> Result<CoachLaneObligation, CoachLaneError> {
    pending.record(
        CoachLaneStage::Blocked,
        Some(refusal),
        CoachLaneResolution::unresolved(observed_on),
    )
}
