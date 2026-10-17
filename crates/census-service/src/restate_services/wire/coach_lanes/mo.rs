#[cfg(test)]
use census_domain::model::{CoachLaneError, CoachLaneObligation, SchoolYear};
use census_domain::UsJurisdiction;

use super::policy::{ArmBasis, Lane, LanePolicy};
use crate::restate_services::plan::RefusalKind;

pub(super) const MO_ARM: &str = "mtccca";
pub(super) const MO_REFUSAL_SLUG: &str = "mo_missing_coach_sources";
pub(super) const MO_TRUE_MISSING: &str = "`mtccca.org` live membership list (only proto responses held) · `moqualityschools.com/member-listing1.html` · `mocsaa.com` · `dese.mo.gov` + `apps.dese.mo.gov` · `www.mshsaa.org` coach routes (documented refusal — do not fetch).";

const MO_ARM_BASIS: &str = "MTCCCA is the only identified Missouri coach-association surface, and the MSHSAA coach routes are robots-refused while the held school listing publishes no coach role columns";
#[cfg(test)]
const ACCEPTED_BY: &str = "census-ops";
#[cfg(test)]
const ACCEPTANCE_EVENT: &str = "jpx4.accepted_refusal";

pub(super) const POLICY: Lane = Lane {
    jurisdiction: UsJurisdiction::Missouri,
    arm: MO_ARM,
    refusal: (MO_REFUSAL_SLUG, MO_TRUE_MISSING),
    kind: RefusalKind::AccessBlocked,
    basis: ArmBasis::Text(MO_ARM_BASIS),
    policy: LanePolicy::Blocked,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::restate_services::jurisdiction::coach_lane_owed;
    use crate::restate_services::wire::{JurisdictionState, SourcePlan};
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

    #[test]
    fn coach_lane_mo_records_the_work_order_true_missing_reason() -> TestResult {
        let recorded = POLICY.refusal(OBSERVED_ON)?;
        check!(eq; recorded.reason(), MO_TRUE_MISSING);
        check!(eq; recorded.source().id.as_str(), MO_REFUSAL_SLUG);
        check!(eq; recorded.observed_on(), OBSERVED_ON);
        check!(
            WORK_ORDER.contains(MO_TRUE_MISSING),
            "the recorded reason is the work order's Missouri true-missing line"
        );
        Ok(())
    }

    #[test]
    fn coach_lane_mo_lane_records_the_refusal_without_closing_the_row() -> TestResult {
        let mut state = JurisdictionState::default();
        check!(coach_lane_owed(&state));
        let row = super::super::evaluate(&POLICY, season()?, 1, OBSERVED_ON)?;
        check!(eq; row.id(), "coach_lane:MO:2026:1");
        check!(eq; row.status(), CoachLaneStatus::Blocked);
        check!(!row.is_terminal());
        check!(eq; row.arm_name(), MO_ARM);
        check!(row.refusal().is_some());
        check!(row.acceptance().is_none());
        check!(row.evidence().is_none());
        state.coach_lane = Some(row);
        check!(coach_lane_owed(&state));
        Ok(())
    }

    #[test]
    fn coach_lane_mo_owner_acceptance_closes_the_row() -> TestResult {
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
    fn coach_lane_mo_lane_matches_the_shared_registry_signature() -> TestResult {
        let entry: fn(&Lane, SchoolYear, u32, &str) -> Result<CoachLaneObligation, CoachLaneError> = super::super::evaluate;
        check!(eq; entry(&POLICY, season()?, 1, OBSERVED_ON)?.id(), "coach_lane:MO:2026:1");
        Ok(())
    }

    #[test]
    fn coach_lane_mo_refusal_survives_reopen_and_replay_without_acceptance() -> TestResult {
        use super::super::bind;
        use census_store::{Store, Table};

        let root = tempfile::tempdir()?;
        let (plan, obligation) = bound()?;
        let mut state = JurisdictionState {
            identity: "jurisdiction:MO:2026-27:1".to_string(),
            plan: Some(plan),
            coach_lane: Some(obligation.clone()),
            ..JurisdictionState::default()
        };
        let expected = serde_json::to_value(&state)?;
        let phase = "mo-coach-refusal-qualification";
        {
            let store = Store::open(root.path())?;
            persist_refusal(&store, &state, phase)?;
        }
        {
            let store = Store::open(root.path())?;
            state = serde_json::from_value(
                store
                    .journal_payload(phase, &state.identity)?
                    .ok_or("MO state missing")?,
            )?;
            check!(eq; store.scan::<CoachLaneObligation>(Table::CoachLanes)?, vec![obligation]);
            for _ in 0..3 {
                state.coach_lane = bind(
                    state.plan.as_mut().ok_or("MO plan missing")?,
                    UsJurisdiction::Missouri,
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

    fn bound() -> Result<(SourcePlan, CoachLaneObligation), Box<dyn std::error::Error>> {
        use super::super::bind;
        use crate::restate_services::plan::{plan as planned, BrowserLaneState};

        let mut plan = SourcePlan::of(
            &planned(UsJurisdiction::Missouri, BrowserLaneState::Absent),
            "fixture".to_string(),
        );
        let obligation = bind(
            &mut plan,
            UsJurisdiction::Missouri,
            season()?,
            1,
            OBSERVED_ON,
        )?
        .ok_or("MO lane missing")?;
        Ok((plan, obligation))
    }

    fn persist_refusal(
        store: &census_store::Store,
        state: &JurisdictionState,
        phase: &str,
    ) -> TestResult {
        use crate::restate_services::plan::RefusalKind;

        let row = state.coach_lane.as_ref().ok_or("MO row missing")?;
        check!(eq; row.status(), CoachLaneStatus::Blocked);
        check!(row.acceptance().is_none());
        check!(row.evidence().is_none());
        check!(coach_lane_owed(state));
        let entries: Vec<_> = state
            .plan
            .as_ref()
            .ok_or("MO plan missing")?
            .refused
            .iter()
            .filter(|entry| entry.slug == MO_REFUSAL_SLUG)
            .collect();
        check!(eq; entries.len(), 1);
        let entry = entries.first().ok_or("MO refusal missing")?;
        check!(eq; entry.reason.as_str(), MO_TRUE_MISSING);
        check!(eq; entry.kind, RefusalKind::AccessBlocked);
        store.replace(census_store::Table::CoachLanes, row)?;
        store.journal_done(phase, &state.identity, state)?;
        store.flush()?;
        Ok(())
    }
}
