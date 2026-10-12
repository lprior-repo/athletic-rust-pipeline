#[cfg(test)]
use census_domain::model::CoachLaneAcceptance;
use census_domain::model::{
    CoachLaneError, CoachLaneObligation, CoachLaneRefusal, CoachLaneResolution, CoachLaneStage,
    PendingCoachLane, SchoolYear, SourceRef,
};
use census_domain::UsJurisdiction;

pub(super) const NV_ARM: &str = "niaa";
pub(super) const NV_REFUSAL_SLUG: &str = "nv_missing_coach_sources";
pub(super) const NV_TRUE_MISSING: &str = "Clark County School District school-site staff directories (ccsd.net sites; SRC-177 Washoe lane same gap) · NIAA publications/prestosports route (SRC-182) + `niaa-coaches` page is empty (0 B) → NIAA coach listing needed · Nevada private-school directory (SRC-180) for non-NIAA schools";

const NV_ARM_BASIS: &str = "Nevada has no registered coach-capable descriptor: `niaa` is the association's held route but no registry entry admits a Nevada coach lane, the `niaa-coaches` page was captured empty (0 B), and the held Southern Nevada coach CSVs are extract artifacts without a durable jurisdiction research frontier";
#[cfg(test)]
const ACCEPTED_BY: &str = "census-ops";
#[cfg(test)]
const ACCEPTANCE_EVENT: &str = "jpx4.accepted_refusal";

pub(super) fn refusal(observed_on: &str) -> Result<CoachLaneRefusal, CoachLaneError> {
    CoachLaneRefusal::new(SourceRef::id(NV_REFUSAL_SLUG), NV_TRUE_MISSING, observed_on)
}

pub(super) fn lane(
    season: SchoolYear,
    revision: u32,
    observed_on: &str,
) -> Result<CoachLaneObligation, CoachLaneError> {
    let pending = CoachLaneObligation::pending(PendingCoachLane {
        jurisdiction: UsJurisdiction::Nevada,
        season,
        revision,
        arm: NV_ARM.to_string(),
        arm_basis: NV_ARM_BASIS.to_string(),
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
        "/../../research/sources/coach-coverage-bundle-20261004/state-work-orders/group-c.md"
    ));

    fn season() -> Result<SchoolYear, Box<dyn std::error::Error>> {
        Ok(SchoolYear::new(2026).ok_or("school year 2026")?)
    }

    fn bound() -> Result<(SourcePlan, CoachLaneObligation), Box<dyn std::error::Error>> {
        let mut plan = SourcePlan::of(
            &planned(UsJurisdiction::Nevada, BrowserLaneState::Absent),
            "fixture".to_string(),
        );
        let obligation = bind(&mut plan, UsJurisdiction::Nevada, season()?, 1, OBSERVED_ON)?
            .ok_or("the Nevada lane is registered")?;
        Ok((plan, obligation))
    }

    fn coach_routes() -> Vec<&'static str> {
        applicable_sources(UsJurisdiction::Nevada)
            .into_iter()
            .filter(|descriptor| descriptor.capabilities.coach_directory)
            .map(|descriptor| descriptor.slug)
            .collect()
    }

    #[test]
    fn coach_lane_nv_records_the_work_order_refusal_without_closing() -> TestResult {
        let recorded = lane(season()?, 1, OBSERVED_ON)?;
        check!(eq; recorded.id(), "coach_lane:NV:2026:1");
        check!(eq; recorded.status(), CoachLaneStatus::Blocked);
        check!(!recorded.is_terminal());
        check!(eq; recorded.arm_name(), NV_ARM);
        check!(recorded.evidence().is_none());
        check!(recorded.acceptance().is_none());
        let refusal = recorded
            .refusal()
            .ok_or("a recorded refusal carries its source and reason")?;
        check!(eq; refusal.source().id.as_str(), NV_REFUSAL_SLUG);
        check!(eq; refusal.reason(), NV_TRUE_MISSING);
        check!(eq; refusal.observed_on(), OBSERVED_ON);
        let mut state = JurisdictionState::default();
        check!(coach_lane_owed(&state));
        state.coach_lane = Some(recorded);
        check!(coach_lane_owed(&state));
        Ok(())
    }

    #[test]
    fn coach_lane_nv_refusal_is_recorded_in_the_durable_plan_path() -> TestResult {
        let (plan, recorded) = bound()?;
        check!(plan
            .refused
            .iter()
            .any(|entry| entry.slug == NV_REFUSAL_SLUG
                && entry.reason == NV_TRUE_MISSING
                && entry.kind == RefusalKind::EngineeringGap));
        let refusals = plan.coach_lane_refusals(OBSERVED_ON)?;
        check!(refusals
            .iter()
            .any(|refusal| refusal.source().id == NV_REFUSAL_SLUG
                && refusal.reason() == NV_TRUE_MISSING));
        check!(eq; recorded.arm_name(), NV_ARM);
        check!(LANES
            .iter()
            .any(|row| row.jurisdiction == UsJurisdiction::Nevada));
        Ok(())
    }

    #[test]
    fn coach_lane_nv_accepted_refusal_closes_the_lane_with_the_explicit_event() -> TestResult {
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

    #[test]
    fn coach_lane_nv_arm_route_admits_no_coach_lane() -> TestResult {
        check!(eq; Dispatch::of(NV_ARM), Dispatch::Unwired);
        check!(teams_arms::arm_for(NV_ARM).is_none());
        let routes = coach_routes();
        check!(
            routes.is_empty(),
            "the registry admits coach routes for Nevada: {routes:?}"
        );
        Ok(())
    }

    #[test]
    fn coach_lane_nv_remainder_is_the_work_order_true_missing() -> TestResult {
        for fragment in NV_TRUE_MISSING.split(" · ") {
            check!(
                WORK_ORDER.contains(fragment),
                "the group-c work order does not carry this bullet: {fragment}"
            );
        }
        Ok(())
    }

    #[test]
    fn coach_lane_nv_refusal_survives_reopen_and_replay_without_acceptance() -> TestResult {
        use census_store::{Store, Table};

        let root = tempfile::tempdir()?;
        let (plan, obligation) = bound()?;
        let mut state = JurisdictionState {
            identity: "jurisdiction:NV:2026-27:1".to_string(),
            plan: Some(plan),
            coach_lane: Some(obligation.clone()),
            ..JurisdictionState::default()
        };
        let expected = serde_json::to_value(&state)?;
        let phase = "nv-coach-refusal-qualification";
        {
            let store = Store::open(root.path())?;
            persist_refusal(&store, &state, phase)?;
        }
        {
            let store = Store::open(root.path())?;
            state = serde_json::from_value(
                store
                    .journal_payload(phase, &state.identity)?
                    .ok_or("NV state missing")?,
            )?;
            check!(eq; store.scan::<CoachLaneObligation>(Table::CoachLanes)?, vec![obligation]);
            for _ in 0..3 {
                state.coach_lane = bind(
                    state.plan.as_mut().ok_or("NV plan missing")?,
                    UsJurisdiction::Nevada,
                    season()?,
                    1,
                    OBSERVED_ON,
                )?;
                check!(eq; serde_json::to_value(&state)?, expected);
            }
            persist_refusal(&store, &state, phase)?;
        }
        let store = Store::open(root.path())?;
        check!(eq; store.journal_payload(phase, &state.identity)?, Some(expected));
        check!(eq; store.journal_keys(phase)?.len(), 1);
        check!(eq; store.walk_table(Table::CoachLanes)?.rows, 1);
        check!(eq; store.walk_table(Table::Coaches)?.rows, 0);
        Ok(())
    }

    fn persist_refusal(
        store: &census_store::Store,
        state: &JurisdictionState,
        phase: &str,
    ) -> TestResult {
        let row = state.coach_lane.as_ref().ok_or("NV row missing")?;
        check!(eq; row.status(), CoachLaneStatus::Blocked);
        check!(row.acceptance().is_none());
        check!(row.evidence().is_none());
        check!(coach_lane_owed(state));
        let entries: Vec<_> = state
            .plan
            .as_ref()
            .ok_or("NV plan missing")?
            .refused
            .iter()
            .filter(|entry| entry.slug == NV_REFUSAL_SLUG)
            .collect();
        check!(eq; entries.len(), 1);
        let entry = entries.first().ok_or("NV refusal missing")?;
        check!(eq; entry.reason.as_str(), NV_TRUE_MISSING);
        check!(eq; entry.kind, RefusalKind::EngineeringGap);
        store.replace(census_store::Table::CoachLanes, row)?;
        store.journal_done(phase, &state.identity, state)?;
        store.flush()?;
        Ok(())
    }
}
