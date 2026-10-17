#[cfg(test)]
use census_domain::model::{CoachLaneError, CoachLaneObligation, SchoolYear};
use census_domain::UsJurisdiction;

use super::policy::{ArmBasis, Lane, LanePolicy};
use crate::restate_services::plan::RefusalKind;

pub(super) const MA_ARM: &str = "mstca";
pub(super) const MA_REFUSAL_SLUG: &str = "ma_missing_coach_sources";
pub(super) const MA_TRUE_MISSING: &str = "MSTCA membership **slice 300–1417** (only 0–300 held) · `profiles.doe.mass.edu` (SRC-072) · `mass.gov` MassGIS + `services1.arcgis.com` (SRC-073/074) · `nepsac.org` (SRC-092) · NEPSTA (SRC-095) · school athletic sites for email (matrix email 20).";

const MA_ARM_BASIS: &str = "the MSTCA member export carries mailboxes but no role or sport column, and the MIAA membership material stops at principal and athletic director";
#[cfg(test)]
const ACCEPTED_BY: &str = "census-ops";
#[cfg(test)]
const ACCEPTANCE_EVENT: &str = "jpx4.accepted_refusal";

pub(super) const POLICY: Lane = Lane {
    jurisdiction: UsJurisdiction::Massachusetts,
    arm: MA_ARM,
    refusal: (MA_REFUSAL_SLUG, MA_TRUE_MISSING),
    kind: RefusalKind::EngineeringGap,
    basis: ArmBasis::Text(MA_ARM_BASIS),
    policy: LanePolicy::Blocked,
};

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
            &planned(UsJurisdiction::Massachusetts, BrowserLaneState::Absent),
            "fixture".to_string(),
        );
        let obligation = bind(
            &mut plan,
            UsJurisdiction::Massachusetts,
            season()?,
            1,
            OBSERVED_ON,
        )?
        .ok_or("the Massachusetts lane is registered")?;
        Ok((plan, obligation))
    }

    fn coach_routes() -> Vec<&'static str> {
        applicable_sources(UsJurisdiction::Massachusetts)
            .into_iter()
            .filter(|descriptor| descriptor.capabilities.coach_directory)
            .map(|descriptor| descriptor.slug)
            .collect()
    }

    #[test]
    fn coach_lane_ma_records_the_work_order_true_missing_reason() -> TestResult {
        let recorded = POLICY.refusal(OBSERVED_ON)?;
        check!(eq; recorded.reason(), MA_TRUE_MISSING);
        check!(eq; recorded.source().id.as_str(), MA_REFUSAL_SLUG);
        check!(eq; recorded.observed_on(), OBSERVED_ON);
        check!(
            WORK_ORDER.contains(MA_TRUE_MISSING),
            "the recorded reason is the work order's Massachusetts true-missing line"
        );
        Ok(())
    }

    #[test]
    fn coach_lane_ma_lane_records_the_refusal_without_closing_the_row() -> TestResult {
        let mut state = JurisdictionState::default();
        check!(coach_lane_owed(&state));
        let row = super::super::evaluate(&POLICY, season()?, 1, OBSERVED_ON)?;
        check!(eq; row.id(), "coach_lane:MA:2026:1");
        check!(eq; row.status(), CoachLaneStatus::Blocked);
        check!(!row.is_terminal());
        check!(eq; row.arm_name(), MA_ARM);
        check!(row.refusal().is_some());
        check!(row.acceptance().is_none());
        check!(row.evidence().is_none());
        state.coach_lane = Some(row);
        check!(coach_lane_owed(&state));
        Ok(())
    }

    #[test]
    fn coach_lane_ma_owner_acceptance_closes_the_row() -> TestResult {
        let mut state = JurisdictionState::default();
        let closed = super::super::accepted(&POLICY, season()?, 1, OBSERVED_ON)?;
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
    fn coach_lane_ma_lane_matches_the_shared_registry_signature() -> TestResult {
        let entry: fn(SchoolYear, u32, &str) -> Result<CoachLaneObligation, CoachLaneError> = lane;
        check!(eq; entry(season()?, 1, OBSERVED_ON)?.id(), "coach_lane:MA:2026:1");
        Ok(())
    }

    #[test]
    fn coach_lane_ma_refusal_is_recorded_in_the_durable_plan_path() -> TestResult {
        let (plan, recorded) = bound()?;
        check!(plan
            .refused
            .iter()
            .any(|entry| entry.slug == MA_REFUSAL_SLUG
                && entry.reason == MA_TRUE_MISSING
                && entry.kind == RefusalKind::EngineeringGap));
        let refusals = plan.coach_lane_refusals(OBSERVED_ON)?;
        check!(refusals
            .iter()
            .any(|refusal| refusal.source().id == MA_REFUSAL_SLUG
                && refusal.reason() == MA_TRUE_MISSING));
        check!(eq; recorded.arm_name(), MA_ARM);
        check!(LANES
            .iter()
            .any(|row| row.jurisdiction == UsJurisdiction::Massachusetts));
        Ok(())
    }

    #[test]
    fn coach_lane_ma_arm_route_admits_no_coach_lane() -> TestResult {
        check!(eq; Dispatch::of(MA_ARM), Dispatch::Unwired);
        check!(teams_arms::arm_for(MA_ARM).is_none());
        let routes = coach_routes();
        check!(
            routes.is_empty(),
            "the registry admits coach routes for Massachusetts: {routes:?}"
        );
        Ok(())
    }

    #[test]
    fn coach_lane_ma_remainder_is_the_work_order_true_missing() -> TestResult {
        for fragment in MA_TRUE_MISSING.split(" · ") {
            check!(
                WORK_ORDER.contains(fragment),
                "the group-b work order does not carry this bullet: {fragment}"
            );
        }
        Ok(())
    }
}
