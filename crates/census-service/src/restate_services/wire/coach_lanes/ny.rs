use census_domain::model::{
    CoachLaneError, CoachLaneObligation, CoachLaneRefusal, CoachLaneResolution, CoachLaneStage,
    PendingCoachLane, SchoolYear, SourceRef,
};
use census_domain::UsJurisdiction;

pub(super) const NY_ARM: &str = "milesplit";
pub(super) const NY_REFUSAL_SLUG: &str = "ny_missing_coach_sources";
pub(super) const NY_TRUE_MISSING: &str = "NYSED SEDREF (`p12.nysed.gov`); Section V (`sectionv.org`); Section IV (`sectionivathletics`); Section IX (`sectionixathletics`); NYSAIS (`nysais.org`); CHSAA NY (`chsaany.org`); NEPSAC (`nepsac.org`); Section II/III/VI/X/XI pages.";

const NY_ARM_BASIS: &str = "the wired milesplit arm is the only New York coach-name surface the plan sweeps and the held NYSPHSAA, PSAL and milesplit bodies carry names and URLs without coach mailboxes, so the lane records the work order's true-missing set instead of arming";

pub(super) fn lane(
    season: SchoolYear,
    revision: u32,
    observed_on: &str,
) -> Result<CoachLaneObligation, CoachLaneError> {
    let pending = CoachLaneObligation::pending(PendingCoachLane {
        jurisdiction: UsJurisdiction::NewYork,
        season,
        revision,
        arm: NY_ARM.to_string(),
        arm_basis: NY_ARM_BASIS.to_string(),
        observed_on: observed_on.to_string(),
    })?;
    pending.record(
        CoachLaneStage::Blocked,
        Some(refusal(observed_on)?),
        CoachLaneResolution::unresolved(observed_on),
    )
}

pub(super) fn refusal(observed_on: &str) -> Result<CoachLaneRefusal, CoachLaneError> {
    CoachLaneRefusal::new(SourceRef::id(NY_REFUSAL_SLUG), NY_TRUE_MISSING, observed_on)
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
            jurisdiction: UsJurisdiction::NewYork,
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
            &planned(UsJurisdiction::NewYork, BrowserLaneState::Absent),
            "fixture".to_string(),
        );
        let obligation = bind(
            &mut plan,
            UsJurisdiction::NewYork,
            season()?,
            1,
            OBSERVED_ON,
        )?
        .ok_or("the New York lane is registered")?;
        Ok((plan, obligation))
    }

    #[test]
    fn coach_lane_ny_records_the_true_missing_refusal_without_closing() -> TestResult {
        let (plan, recorded) = bound()?;
        check!(eq; recorded.id(), "coach_lane:NY:2026:1");
        check!(eq; recorded.status(), CoachLaneStatus::Blocked);
        check!(!recorded.is_terminal());
        check!(eq; recorded.arm_name(), NY_ARM);
        check!(recorded.evidence().is_none());
        check!(recorded.acceptance().is_none());
        let refusal = recorded
            .refusal()
            .ok_or("a recorded refusal carries its source and reason")?;
        check!(eq; refusal.source().id.as_str(), NY_REFUSAL_SLUG);
        check!(eq; refusal.reason(), NY_TRUE_MISSING);
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
            .any(|reason| reason.contains(NY_REFUSAL_SLUG)));
        check!(reasons
            .iter()
            .any(|reason| reason.contains(NY_TRUE_MISSING)));
        Ok(())
    }

    #[test]
    fn coach_lane_ny_true_missing_reason_is_the_work_order_line() -> TestResult {
        check!(WORK_ORDER.contains(NY_TRUE_MISSING));
        check!(WORK_ORDER.contains("NYSED SEDREF"));
        Ok(())
    }

    #[test]
    fn coach_lane_ny_refusal_is_recorded_in_the_durable_plan_path() -> TestResult {
        let (plan, recorded) = bound()?;
        check!(plan.has_engineering_gaps());
        check!(plan
            .refused
            .iter()
            .any(|entry| entry.slug == NY_REFUSAL_SLUG && entry.reason == NY_TRUE_MISSING));
        let refusals = plan.coach_lane_refusals(OBSERVED_ON)?;
        check!(refusals
            .iter()
            .any(|refusal| refusal.source().id == NY_REFUSAL_SLUG
                && refusal.reason() == NY_TRUE_MISSING));
        check!(eq; recorded.arm_name(), NY_ARM);
        Ok(())
    }

    #[test]
    fn coach_lane_ny_arm_is_the_wired_milesplit_source() -> TestResult {
        let (plan, _) = bound()?;
        check!(teams_arms::arm_for(NY_ARM).is_some());
        check!(plan.sweepable.iter().any(|slug| slug == NY_ARM));
        Ok(())
    }
}

#[cfg(test)]
#[path = "qualification_tests.rs"]
mod qualification_tests;
