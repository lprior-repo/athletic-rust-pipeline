use census_domain::model::{
    CoachLaneError, CoachLaneObligation, CoachLaneRefusal, CoachLaneResolution, CoachLaneStage,
    PendingCoachLane, SchoolYear, SourceRef,
};
use census_domain::UsJurisdiction;

pub(super) const AZ_ARM: &str = "aia";
pub(super) const AZ_REFUSAL_SLUG: &str = "az_missing_coach_sources";
pub(super) const AZ_TRUE_MISSING: &str = "AIA member/sport-coach directory route (`aiaonline.org/schools`); AIA authenticated admin email directory (`admin.aiaonline.org`, restricted); CAA (`azcaa.com`); CAA Bound (gobound refused — documented); ACEC (`azchristianschools`); AZPreps365 team pages.";

const AZ_ARM_BASIS: &str = "`aia` is wired and sweepable for Arizona; the search JSON is school-level and the qualified school profile fixtures publish TF/XC head-coach names but no public coach mailboxes, while the administrative email directory requires login, so the lane records the work order's true-missing set instead of arming";

pub(super) fn lane(
    season: SchoolYear,
    revision: u32,
    observed_on: &str,
) -> Result<CoachLaneObligation, CoachLaneError> {
    let pending = CoachLaneObligation::pending(PendingCoachLane {
        jurisdiction: UsJurisdiction::Arizona,
        season,
        revision,
        arm: AZ_ARM.to_string(),
        arm_basis: AZ_ARM_BASIS.to_string(),
        observed_on: observed_on.to_string(),
    })?;
    pending.record(
        CoachLaneStage::Blocked,
        Some(refusal(observed_on)?),
        CoachLaneResolution::unresolved(observed_on),
    )
}

pub(super) fn refusal(observed_on: &str) -> Result<CoachLaneRefusal, CoachLaneError> {
    CoachLaneRefusal::new(SourceRef::id(AZ_REFUSAL_SLUG), AZ_TRUE_MISSING, observed_on)
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
            jurisdiction: UsJurisdiction::Arizona,
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
            &planned(UsJurisdiction::Arizona, BrowserLaneState::Absent),
            "fixture".to_string(),
        );
        let obligation = bind(
            &mut plan,
            UsJurisdiction::Arizona,
            season()?,
            1,
            OBSERVED_ON,
        )?
        .ok_or("the Arizona lane is registered")?;
        Ok((plan, obligation))
    }

    #[test]
    fn coach_lane_az_records_the_true_missing_refusal_without_closing() -> TestResult {
        let (plan, recorded) = bound()?;
        check!(eq; recorded.id(), "coach_lane:AZ:2026:1");
        check!(eq; recorded.status(), CoachLaneStatus::Blocked);
        check!(!recorded.is_terminal());
        check!(eq; recorded.arm_name(), AZ_ARM);
        check!(recorded.evidence().is_none());
        check!(recorded.acceptance().is_none());
        let refusal = recorded
            .refusal()
            .ok_or("a recorded refusal carries its source and reason")?;
        check!(eq; refusal.source().id.as_str(), AZ_REFUSAL_SLUG);
        check!(eq; refusal.reason(), AZ_TRUE_MISSING);
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
            .any(|reason| reason.contains(AZ_REFUSAL_SLUG)));
        check!(reasons
            .iter()
            .any(|reason| reason.contains(AZ_TRUE_MISSING)));
        Ok(())
    }

    #[test]
    fn coach_lane_az_true_missing_reason_is_the_work_order_line() -> TestResult {
        check!(WORK_ORDER.contains(AZ_TRUE_MISSING));
        check!(WORK_ORDER.contains("AIA member/sport-coach directory route"));
        Ok(())
    }

    #[test]
    fn coach_lane_az_refusal_is_recorded_in_the_durable_plan_path() -> TestResult {
        let (plan, recorded) = bound()?;
        check!(plan.has_engineering_gaps());
        check!(plan
            .refused
            .iter()
            .any(|entry| entry.slug == AZ_REFUSAL_SLUG && entry.reason == AZ_TRUE_MISSING));
        let refusals = plan.coach_lane_refusals(OBSERVED_ON)?;
        check!(refusals
            .iter()
            .any(|refusal| refusal.source().id == AZ_REFUSAL_SLUG
                && refusal.reason() == AZ_TRUE_MISSING));
        check!(eq; recorded.arm_name(), AZ_ARM);
        Ok(())
    }

    #[test]
    fn coach_lane_az_arm_is_the_wired_aia_source() -> TestResult {
        let (plan, recorded) = bound()?;
        check!(teams_arms::arm_for(AZ_ARM).is_some());
        check!(plan.sweepable.iter().any(|slug| slug == AZ_ARM));
        check!(eq; recorded.status(), CoachLaneStatus::Blocked);
        check!(recorded.evidence().is_none());
        Ok(())
    }
}
