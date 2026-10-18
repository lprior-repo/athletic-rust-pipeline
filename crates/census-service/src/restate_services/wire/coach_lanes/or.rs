#[cfg(test)]
use census_domain::model::{CoachLaneObligation, SchoolYear};
use census_domain::UsJurisdiction;

use super::policy::{ArmBasis, Lane, LanePolicy};
use crate::restate_services::plan::RefusalKind;

pub(super) const OR_ARM: &str = "coach_contacts";
pub(super) const OR_REFUSAL_SLUG: &str = "or_missing_coach_sources";
pub(super) const OR_TRUE_MISSING: &str = "OSAA index has 299 members; only per-school pages for a handful beyond the 298 JSONL rows may be missing (verify count) · Private/independent schools outside OSAA membership (Oregon Federation of Independent Schools, SRC-148 — `ofisweb.org` not held) · OACA (oregoncoach.org, SRC-150/151) association directory not held";

const OR_ARM_BASIS: &str = "`coach_contacts` is the only Oregon descriptor that declares both school-coach contact and an admitted jurisdiction, and the plan refuses it as an engineering gap: the researched 298-profile corpus lives in the prototype JSONL, not in a durable jurisdiction research frontier";
#[cfg(test)]
const ACCEPTED_BY: &str = "census-ops";
#[cfg(test)]
const ACCEPTANCE_EVENT: &str = "jpx4.accepted_refusal";

pub(super) const POLICY: Lane = Lane {
    jurisdiction: UsJurisdiction::Oregon,
    arm: OR_ARM,
    refusal: (OR_REFUSAL_SLUG, OR_TRUE_MISSING),
    kind: RefusalKind::EngineeringGap,
    basis: ArmBasis::Text(OR_ARM_BASIS),
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
        "/../../research/sources/coach-coverage-bundle-20261004/state-work-orders/group-c.md"
    ));

    fn season() -> Result<SchoolYear, Box<dyn std::error::Error>> {
        Ok(SchoolYear::new(2026).ok_or("school year 2026")?)
    }

    fn bound() -> Result<(SourcePlan, CoachLaneObligation), Box<dyn std::error::Error>> {
        let mut plan = SourcePlan::of(
            &planned(UsJurisdiction::Oregon, BrowserLaneState::Absent),
            "fixture".to_string(),
        );
        let obligation = bind(&mut plan, UsJurisdiction::Oregon, season()?, 1, OBSERVED_ON)?
            .ok_or("the Oregon lane is registered")?;
        Ok((plan, obligation))
    }

    fn coach_routes() -> Vec<&'static str> {
        applicable_sources(UsJurisdiction::Oregon)
            .into_iter()
            .filter(|descriptor| descriptor.capabilities.coach_directory)
            .map(|descriptor| descriptor.slug)
            .collect()
    }

    #[test]
    fn coach_lane_or_records_the_work_order_refusal_without_closing() -> TestResult {
        let recorded = super::super::evaluate(&POLICY, season()?, 1, OBSERVED_ON)?;
        check!(eq; recorded.id(), "coach_lane:OR:2026:1");
        check!(eq; recorded.status(), CoachLaneStatus::Blocked);
        check!(!recorded.is_terminal());
        check!(eq; recorded.arm_name(), OR_ARM);
        check!(recorded.evidence().is_none());
        check!(recorded.acceptance().is_none());
        let refusal = recorded
            .refusal()
            .ok_or("a recorded refusal carries its source and reason")?;
        check!(eq; refusal.source().id.as_str(), OR_REFUSAL_SLUG);
        check!(eq; refusal.reason(), OR_TRUE_MISSING);
        check!(eq; refusal.observed_on(), OBSERVED_ON);
        let mut state = JurisdictionState::default();
        check!(coach_lane_owed(&state));
        state.coach_lane = Some(recorded);
        check!(coach_lane_owed(&state));
        Ok(())
    }

    #[test]
    fn coach_lane_or_refusal_is_recorded_in_the_durable_plan_path() -> TestResult {
        let (plan, recorded) = bound()?;
        check!(plan
            .refused
            .iter()
            .any(|entry| entry.slug == OR_REFUSAL_SLUG
                && entry.reason == OR_TRUE_MISSING
                && entry.kind == RefusalKind::EngineeringGap));
        let refusals = plan.coach_lane_refusals(OBSERVED_ON)?;
        check!(refusals
            .iter()
            .any(|refusal| refusal.source().id == OR_REFUSAL_SLUG
                && refusal.reason() == OR_TRUE_MISSING));
        check!(eq; recorded.arm_name(), OR_ARM);
        check!(LANES
            .iter()
            .any(|row| row.jurisdiction == UsJurisdiction::Oregon));
        Ok(())
    }

    #[test]
    fn coach_lane_or_accepted_refusal_closes_the_lane_with_the_explicit_event() -> TestResult {
        let closing = super::super::accepted(&POLICY, season()?, 1, OBSERVED_ON)?;
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

    #[test]
    fn coach_lane_or_registered_route_is_not_a_wired_arm() -> TestResult {
        check!(eq; Dispatch::of(OR_ARM), Dispatch::Unwired);
        check!(teams_arms::arm_for(OR_ARM).is_none());
        let routes = coach_routes();
        check!(
            routes.contains(&OR_ARM),
            "the registry no longer admits {OR_ARM} for Oregon: {routes:?}"
        );
        for slug in routes {
            check!(
                teams_arms::arm_for(slug).is_none(),
                "{slug} is wired: the Oregon refusal must be revisited"
            );
        }
        Ok(())
    }

    #[test]
    fn coach_lane_or_remainder_is_the_work_order_true_missing() -> TestResult {
        for fragment in OR_TRUE_MISSING.split(" · ") {
            check!(
                WORK_ORDER.contains(fragment),
                "the group-c work order does not carry this bullet: {fragment}"
            );
        }
        Ok(())
    }

    #[test]
    fn coach_lane_or_refusal_survives_store_reopen_and_rebind() -> TestResult {
        use census_store::{Store, Table};

        let root = tempfile::tempdir()?;
        let (plan, recorded) = bound()?;
        let state = JurisdictionState {
            identity: recorded.id().to_string(),
            plan: Some(plan),
            coach_lane: Some(recorded.clone()),
            ..JurisdictionState::default()
        };
        {
            let store = Store::open(root.path())?;
            store.replace(Table::CoachLanes, &recorded)?;
            store.journal_done("or-coach-qualification", &state.identity, &state)?;
            store.flush()?;
        }
        let store = Store::open(root.path())?;
        check!(eq; store.scan::<CoachLaneObligation>(Table::CoachLanes)?, vec![recorded]);
        let payload = store
            .journal_payload("or-coach-qualification", &state.identity)?
            .ok_or("Oregon refusal lost on reopen")?;
        let mut retained: JurisdictionState = serde_json::from_value(payload)?;
        check!(eq; serde_json::to_value(&retained)?, serde_json::to_value(&state)?);
        let plan = retained.plan.as_mut().ok_or("Oregon plan missing")?;
        retained.coach_lane = bind(plan, UsJurisdiction::Oregon, season()?, 1, OBSERVED_ON)?;
        check!(eq; serde_json::to_value(&retained)?, serde_json::to_value(&state)?);
        check!(coach_lane_owed(&retained));
        let refusal = retained
            .coach_lane
            .as_ref()
            .and_then(CoachLaneObligation::refusal)
            .ok_or("Oregon refusal missing")?;
        check!(eq; refusal.reason(), OR_TRUE_MISSING);
        check!(eq; refusal.source().id.as_str(), OR_REFUSAL_SLUG);
        check!(eq; store.journal_keys("or-coach-qualification")?.len(), 1);
        check!(eq; store.walk_table(Table::Coaches)?.rows, 0);
        Ok(())
    }
}
