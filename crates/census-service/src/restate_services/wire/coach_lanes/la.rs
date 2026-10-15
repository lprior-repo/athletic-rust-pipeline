#[cfg(test)]
use census_domain::model::CoachLaneAcceptance;
use census_domain::model::{
    CoachLaneError, CoachLaneObligation, CoachLaneRefusal, CoachLaneResolution, CoachLaneStage,
    PendingCoachLane, SchoolYear, SourceRef,
};
use census_domain::UsJurisdiction;

pub(super) const LA_ARM: &str = "milesplit";
pub(super) const LA_REFUSAL_SLUG: &str = "la_missing_coach_sources";
pub(super) const LA_TRUE_MISSING: &str = "LHSCA (`lhsaa.org/lhsca`); ACEL (`theacel.com`); Louisiana School Finder (`louisianaschools.com`); DirectAthletics LA teams (`directathletics.com` route); school sites (emails).";

const LA_ARM_BASIS: &str = "the wired milesplit arm is the only Louisiana surface the plan sweeps with names and the held LHSAA registered-coaches extract (SRC-215, 2025-09-22) carries XC coach names without mailboxes for the completed season while the coaches-directory PDF route is a documented text-extraction refusal, so the lane records the work order's true-missing set instead of arming";
#[cfg(test)]
const ACCEPTED_BY: &str = "census-ops";
#[cfg(test)]
const ACCEPTANCE_EVENT: &str = "jpx4.accepted_refusal";

pub(super) fn lane(
    season: SchoolYear,
    revision: u32,
    observed_on: &str,
) -> Result<CoachLaneObligation, CoachLaneError> {
    let pending = CoachLaneObligation::pending(PendingCoachLane {
        jurisdiction: UsJurisdiction::Louisiana,
        season,
        revision,
        arm: LA_ARM.to_string(),
        arm_basis: LA_ARM_BASIS.to_string(),
        observed_on: observed_on.to_string(),
    })?;
    pending.record(
        CoachLaneStage::Blocked,
        Some(refusal(observed_on)?),
        CoachLaneResolution::unresolved(observed_on),
    )
}
#[cfg(test)]
pub(super) fn accepted(
    season: SchoolYear,
    revision: u32,
    observed_on: &str,
) -> Result<CoachLaneObligation, CoachLaneError> {
    let recorded = lane(season, revision, observed_on)?;
    let acceptance = CoachLaneAcceptance::new(ACCEPTED_BY, ACCEPTANCE_EVENT, observed_on)?;
    recorded.accept_refusal(acceptance)
}

pub(super) fn refusal(observed_on: &str) -> Result<CoachLaneRefusal, CoachLaneError> {
    CoachLaneRefusal::new(SourceRef::id(LA_REFUSAL_SLUG), LA_TRUE_MISSING, observed_on)
}

#[cfg(test)]
mod tests {
    use super::super::bind;
    use super::*;
    use crate::restate_services::jurisdiction::{coach_lane_owed, owed_work};
    use crate::restate_services::plan::{plan as planned, BrowserLaneState};
    use crate::restate_services::teams_arms;
    use crate::restate_services::wire::{
        HistoryWindow, JurisdictionRequest, JurisdictionState, SourcePlan,
    };
    use census_domain::model::Revision;
    use census_domain::model::{CoachLaneMailbox, CoachLaneStatus};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const OBSERVED_ON: &str = "2026-10-11";
    const WORK_ORDER: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/sources/coach-coverage-bundle-20261004/state-work-orders/group-a.md"
    ));

    fn season() -> Result<SchoolYear, Box<dyn std::error::Error>> {
        Ok(SchoolYear::new(2026).ok_or("school year 2026")?)
    }

    fn request() -> Result<JurisdictionRequest, Box<dyn std::error::Error>> {
        Ok(JurisdictionRequest {
            jurisdiction: UsJurisdiction::Louisiana,
            season: season()?,
            revision: Revision(1),
            history: HistoryWindow::cohort(OBSERVED_ON)?,
            refresh: false,
            limit_per_state: None,
            concurrency: 1,
            observed_on: None,
            authorized_hosts: Vec::new(),
            source_parallelism: 1,
        })
    }

    fn bound() -> Result<(SourcePlan, CoachLaneObligation), Box<dyn std::error::Error>> {
        let mut plan = SourcePlan::of(
            &planned(UsJurisdiction::Louisiana, BrowserLaneState::Absent),
            "fixture".to_string(),
        );
        let obligation = bind(
            &mut plan,
            UsJurisdiction::Louisiana,
            season()?,
            1,
            OBSERVED_ON,
        )?
        .ok_or("the Louisiana lane is registered")?;
        Ok((plan, obligation))
    }

    #[test]
    fn coach_lane_la_records_the_true_missing_refusal_without_closing() -> TestResult {
        let (plan, recorded) = bound()?;
        check!(eq; recorded.id(), "coach_lane:LA:2026:1");
        check!(eq; recorded.status(), CoachLaneStatus::Blocked);
        check!(!recorded.is_terminal());
        check!(eq; recorded.arm_name(), LA_ARM);
        check!(recorded.evidence().is_none());
        check!(recorded.acceptance().is_none());
        let refusal = recorded
            .refusal()
            .ok_or("a recorded refusal carries its source and reason")?;
        check!(eq; refusal.source().id.as_str(), LA_REFUSAL_SLUG);
        check!(eq; refusal.reason(), LA_TRUE_MISSING);
        check!(eq; refusal.observed_on(), OBSERVED_ON);
        check!(eq; recorded.resolution().mailbox, CoachLaneMailbox::Unpublished);
        let mut state = JurisdictionState::default();
        check!(coach_lane_owed(&state));
        state.plan = Some(plan);
        state.coach_lane = Some(recorded);
        check!(coach_lane_owed(&state));
        let (owed, reasons) = owed_work(&state, &request()?);
        check!(owed.iter().any(|stage| stage == "coach_lane"));
        check!(reasons
            .iter()
            .any(|reason| reason.contains(LA_REFUSAL_SLUG)));
        check!(reasons
            .iter()
            .any(|reason| reason.contains(LA_TRUE_MISSING)));
        Ok(())
    }

    #[test]
    fn coach_lane_la_true_missing_reason_is_the_work_order_line() -> TestResult {
        check!(WORK_ORDER.contains(LA_TRUE_MISSING));
        check!(WORK_ORDER.contains("LHSCA"));
        Ok(())
    }

    #[test]
    fn coach_lane_la_refusal_is_recorded_in_the_durable_plan_path() -> TestResult {
        let (plan, recorded) = bound()?;
        check!(plan.has_engineering_gaps());
        check!(plan
            .refused
            .iter()
            .any(|entry| entry.slug == LA_REFUSAL_SLUG && entry.reason == LA_TRUE_MISSING));
        let refusals = plan.coach_lane_refusals(OBSERVED_ON)?;
        check!(refusals
            .iter()
            .any(|refusal| refusal.source().id == LA_REFUSAL_SLUG
                && refusal.reason() == LA_TRUE_MISSING));
        check!(eq; recorded.arm_name(), LA_ARM);
        Ok(())
    }

    #[test]
    fn coach_lane_la_arm_is_the_wired_milesplit_source() -> TestResult {
        let (plan, _) = bound()?;
        check!(teams_arms::arm_for(LA_ARM).is_some());
        check!(plan.sweepable.iter().any(|slug| slug == LA_ARM));
        Ok(())
    }
    #[test]
    fn coach_lane_la_accepted_refusal_closes_the_lane_with_the_explicit_event() -> TestResult {
        let closing = accepted(season()?, 1, OBSERVED_ON)?;
        check!(closing.is_terminal());
        check!(eq; closing.status(), CoachLaneStatus::AcceptedRefusal);
        check!(closing.refusal().is_some());
        let acceptance = closing
            .acceptance()
            .ok_or("a closed refusal carries its acceptance")?;
        check!(eq; acceptance.actor(), ACCEPTED_BY);
        check!(eq; acceptance.event(), ACCEPTANCE_EVENT);
        check!(eq; acceptance.observed_on(), OBSERVED_ON);
        let state = JurisdictionState {
            coach_lane: Some(closing),
            ..JurisdictionState::default()
        };
        check!(!coach_lane_owed(&state));
        Ok(())
    }
}
