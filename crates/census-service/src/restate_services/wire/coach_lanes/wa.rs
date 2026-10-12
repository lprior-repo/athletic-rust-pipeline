#[cfg(test)]
use census_domain::model::CoachLaneAcceptance;
use census_domain::model::{
    CoachLaneError, CoachLaneObligation, CoachLaneRefusal, CoachLaneResolution, CoachLaneStage,
    PendingCoachLane, SchoolYear, SourceRef,
};
use census_domain::UsJurisdiction;

pub(super) const WA_ARM: &str = "milesplit";
pub(super) const WA_REFUSAL_SLUG: &str = "wa_missing_coach_sources";
pub(super) const WA_TRUE_MISSING: &str = "WSCCCA (`wsccca.com`); WSTFCA (`wstfca.com`); WFIS (`wfis.org`); WA SBE private schools (`sbe.wa.gov`); OSPI Education Directory (`eds.ospi.k12.wa.us`); OSPI ArcGIS; KingCo (`kingcoathletics.com`); Seattle Public Schools coach pages (`seattleschools.org`).";

const WA_ARM_BASIS: &str = "the wired milesplit arm is the only Washington surface the plan sweeps with names and the held finalforms rows carry athletic-director names without coach emails while the association bodies are directory and state-partition links, so the lane records the work order's true-missing set instead of arming";
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
        jurisdiction: UsJurisdiction::Washington,
        season,
        revision,
        arm: WA_ARM.to_string(),
        arm_basis: WA_ARM_BASIS.to_string(),
        observed_on: observed_on.to_string(),
    })?;
    pending.record(
        CoachLaneStage::Blocked,
        Some(refusal(observed_on)?),
        CoachLaneResolution::unresolved(observed_on),
    )
}

pub(super) fn refusal(observed_on: &str) -> Result<CoachLaneRefusal, CoachLaneError> {
    CoachLaneRefusal::new(SourceRef::id(WA_REFUSAL_SLUG), WA_TRUE_MISSING, observed_on)
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
    use census_domain::model::{CoachLaneMailbox, CoachLaneStatus};
    use census_reconcile::identity::Revision;

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
            jurisdiction: UsJurisdiction::Washington,
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
            &planned(UsJurisdiction::Washington, BrowserLaneState::Absent),
            "fixture".to_string(),
        );
        let obligation = bind(
            &mut plan,
            UsJurisdiction::Washington,
            season()?,
            1,
            OBSERVED_ON,
        )?
        .ok_or("the Washington lane is registered")?;
        Ok((plan, obligation))
    }

    #[test]
    fn coach_lane_wa_records_the_true_missing_refusal_without_closing() -> TestResult {
        let (plan, recorded) = bound()?;
        check!(eq; recorded.id(), "coach_lane:WA:2026:1");
        check!(eq; recorded.status(), CoachLaneStatus::Blocked);
        check!(!recorded.is_terminal());
        check!(eq; recorded.arm_name(), WA_ARM);
        check!(recorded.evidence().is_none());
        check!(recorded.acceptance().is_none());
        let refusal = recorded
            .refusal()
            .ok_or("a recorded refusal carries its source and reason")?;
        check!(eq; refusal.source().id.as_str(), WA_REFUSAL_SLUG);
        check!(eq; refusal.reason(), WA_TRUE_MISSING);
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
            .any(|reason| reason.contains(WA_REFUSAL_SLUG)));
        check!(reasons
            .iter()
            .any(|reason| reason.contains(WA_TRUE_MISSING)));
        Ok(())
    }

    #[test]
    fn coach_lane_wa_true_missing_reason_is_the_work_order_line() -> TestResult {
        check!(WORK_ORDER.contains(WA_TRUE_MISSING));
        check!(WORK_ORDER.contains("WSCCCA"));
        Ok(())
    }

    #[test]
    fn coach_lane_wa_refusal_is_recorded_in_the_durable_plan_path() -> TestResult {
        let (plan, recorded) = bound()?;
        check!(plan.has_engineering_gaps());
        check!(plan
            .refused
            .iter()
            .any(|entry| entry.slug == WA_REFUSAL_SLUG && entry.reason == WA_TRUE_MISSING));
        let refusals = plan.coach_lane_refusals(OBSERVED_ON)?;
        check!(refusals
            .iter()
            .any(|refusal| refusal.source().id == WA_REFUSAL_SLUG
                && refusal.reason() == WA_TRUE_MISSING));
        check!(eq; recorded.arm_name(), WA_ARM);
        Ok(())
    }

    #[test]
    fn coach_lane_wa_arm_is_the_wired_milesplit_source() -> TestResult {
        let (plan, _) = bound()?;
        check!(teams_arms::arm_for(WA_ARM).is_some());
        check!(plan.sweepable.iter().any(|slug| slug == WA_ARM));
        Ok(())
    }
    #[test]
    fn coach_lane_wa_accepted_refusal_closes_the_lane_with_the_explicit_event() -> TestResult {
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
