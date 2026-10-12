#[cfg(test)]
use census_domain::model::CoachLaneAcceptance;
use census_domain::model::{
    CoachLaneError, CoachLaneObligation, CoachLaneRefusal, CoachLaneResolution, CoachLaneStage,
    PendingCoachLane, SchoolYear, SourceRef,
};
use census_domain::UsJurisdiction;

pub(super) const OK_ARM: &str = "ossaarankings";
pub(super) const OK_REFUSAL_SLUG: &str = "ok_missing_coach_sources";
pub(super) const OK_TRUE_MISSING: &str = "`ohstrack.com` membership directory (none published; info/empty form only) · `facebook.com/OCCTCA` (association contact, E1 #21) · `ncpsa.org/directory/oklahoma/` (55 private) · `heartlandathletics`, `opsac.org`, `ocacademy.org` (DERIVED) · school sites for email; an **adapter** for the OSSAA team-page coach-name route (probe-only today).";

const OK_ARM_BASIS: &str = "the OSSAA team pages label one head coach per team but the route has no adapter, and every held Oklahoma body publishes no coach mailbox";
#[cfg(test)]
const ACCEPTED_BY: &str = "census-ops";
#[cfg(test)]
const ACCEPTANCE_EVENT: &str = "jpx4.accepted_refusal";

pub(super) fn refusal(observed_on: &str) -> Result<CoachLaneRefusal, CoachLaneError> {
    CoachLaneRefusal::new(SourceRef::id(OK_REFUSAL_SLUG), OK_TRUE_MISSING, observed_on)
}

pub(super) fn lane(
    season: SchoolYear,
    revision: u32,
    observed_on: &str,
) -> Result<CoachLaneObligation, CoachLaneError> {
    let pending = CoachLaneObligation::pending(PendingCoachLane {
        jurisdiction: UsJurisdiction::Oklahoma,
        season,
        revision,
        arm: OK_ARM.to_string(),
        arm_basis: OK_ARM_BASIS.to_string(),
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

#[cfg(test)]
mod tests {
    use super::super::{bind, LANES};
    use super::*;
    use crate::restate_services::jurisdiction::coach_lane_owed;
    use crate::restate_services::plan::{plan as planned, BrowserLaneState, Dispatch, RefusalKind};
    use crate::restate_services::teams_arms;
    use crate::restate_services::wire::{JurisdictionState, SourcePlan};
    use census_crawl::applicability::applicable_sources;
    use census_domain::model::CoachLaneStatus;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const OBSERVED_ON: &str = "2026-10-11";
    const WORK_ORDER: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/sources/coach-coverage-bundle-20261004/state-work-orders/group-b.md"
    ));

    fn season() -> Result<SchoolYear, Box<dyn std::error::Error>> {
        Ok(SchoolYear::new(2026).ok_or("school year 2026")?)
    }

    fn bound() -> Result<(SourcePlan, CoachLaneObligation), Box<dyn std::error::Error>> {
        let mut plan = SourcePlan::of(
            &planned(UsJurisdiction::Oklahoma, BrowserLaneState::Absent),
            "fixture".to_string(),
        );
        let obligation = bind(
            &mut plan,
            UsJurisdiction::Oklahoma,
            season()?,
            1,
            OBSERVED_ON,
        )?
        .ok_or("the Oklahoma lane is registered")?;
        Ok((plan, obligation))
    }

    fn coach_routes() -> Vec<&'static str> {
        applicable_sources(UsJurisdiction::Oklahoma)
            .into_iter()
            .filter(|descriptor| descriptor.capabilities.coach_directory)
            .map(|descriptor| descriptor.slug)
            .collect()
    }

    #[test]
    fn coach_lane_ok_records_the_work_order_true_missing_reason() -> TestResult {
        let recorded = refusal(OBSERVED_ON)?;
        check!(eq; recorded.reason(), OK_TRUE_MISSING);
        check!(eq; recorded.source().id.as_str(), OK_REFUSAL_SLUG);
        check!(eq; recorded.observed_on(), OBSERVED_ON);
        check!(
            WORK_ORDER.contains(OK_TRUE_MISSING),
            "the recorded reason is the work order's Oklahoma true-missing line"
        );
        Ok(())
    }

    #[test]
    fn coach_lane_ok_lane_records_the_refusal_without_closing_the_row() -> TestResult {
        let mut state = JurisdictionState::default();
        check!(coach_lane_owed(&state));
        let row = lane(season()?, 1, OBSERVED_ON)?;
        check!(eq; row.id(), "coach_lane:OK:2026:1");
        check!(eq; row.status(), CoachLaneStatus::Blocked);
        check!(!row.is_terminal());
        check!(eq; row.arm_name(), OK_ARM);
        check!(row.refusal().is_some());
        check!(row.acceptance().is_none());
        check!(row.evidence().is_none());
        state.coach_lane = Some(row);
        check!(coach_lane_owed(&state));
        Ok(())
    }

    #[test]
    fn coach_lane_ok_owner_acceptance_closes_the_row() -> TestResult {
        let mut state = JurisdictionState::default();
        let closed = accepted(season()?, 1, OBSERVED_ON)?;
        check!(eq; closed.status(), CoachLaneStatus::AcceptedRefusal);
        check!(closed.is_terminal());
        check!(closed.evidence().is_none());
        let acceptance = closed
            .acceptance()
            .ok_or("an accepted refusal carries one")?;
        check!(eq; acceptance.actor(), ACCEPTED_BY);
        check!(eq; acceptance.event(), ACCEPTANCE_EVENT);
        check!(eq; acceptance.observed_on(), OBSERVED_ON);
        state.coach_lane = Some(closed);
        check!(!coach_lane_owed(&state));
        Ok(())
    }

    #[test]
    fn coach_lane_ok_lane_matches_the_shared_registry_signature() -> TestResult {
        let entry: fn(SchoolYear, u32, &str) -> Result<CoachLaneObligation, CoachLaneError> = lane;
        check!(eq; entry(season()?, 1, OBSERVED_ON)?.id(), "coach_lane:OK:2026:1");
        Ok(())
    }

    #[test]
    fn coach_lane_ok_refusal_is_recorded_in_the_durable_plan_path() -> TestResult {
        let (plan, recorded) = bound()?;
        check!(plan
            .refused
            .iter()
            .any(|entry| entry.slug == OK_REFUSAL_SLUG
                && entry.reason == OK_TRUE_MISSING
                && entry.kind == RefusalKind::EngineeringGap));
        let refusals = plan.coach_lane_refusals(OBSERVED_ON)?;
        check!(refusals
            .iter()
            .any(|refusal| refusal.source().id == OK_REFUSAL_SLUG
                && refusal.reason() == OK_TRUE_MISSING));
        check!(eq; recorded.arm_name(), OK_ARM);
        check!(LANES
            .iter()
            .any(|row| row.jurisdiction == UsJurisdiction::Oklahoma));
        Ok(())
    }

    #[test]
    fn coach_lane_ok_arm_is_unwired_but_arbiter_orgs_is_registered() -> TestResult {
        check!(eq; Dispatch::of(OK_ARM), Dispatch::Unwired);
        check!(teams_arms::arm_for(OK_ARM).is_none());
        let routes = coach_routes();
        check!(
            routes.contains(&"arbiter_orgs"),
            "Oklahoma has the arbiter_orgs route registered (empty member page for org 106940): {routes:?}"
        );
        Ok(())
    }

    #[test]
    fn coach_lane_ok_remainder_is_the_work_order_true_missing() -> TestResult {
        for fragment in OK_TRUE_MISSING.split(" · ") {
            check!(
                WORK_ORDER.contains(fragment),
                "the group-b work order does not carry this bullet: {fragment}"
            );
        }
        Ok(())
    }
}
