#[cfg(test)]
use census_domain::model::{CoachLaneObligation, SchoolYear};
use census_domain::UsJurisdiction;

use super::policy::{ArmBasis, Lane, LanePolicy};
use crate::restate_services::plan::RefusalKind;

pub(super) const TX_ARM: &str = "coach_contacts";
pub(super) const TX_REFUSAL_SLUG: &str = "tx_missing_coach_sources";
pub(super) const TX_TRUE_MISSING: &str = "TEA AskTED personnel downloads (`tealprod.tea.state.tx.us`); TAPPS (`tapps.biz`); TEPSAC (`tepsac.org`); TCAF (`tcafellowship.com`); SPC (`spcsports.org`); TCSAAL (`texascharter.org`); CCCAT (`cccat.org`); TTFCA (`ttfca.org`); TGCA (`austintgca.com`); THSCA (`thsca.com`); Clell Wade Coaches Directory (`coachesdirectory.com`, licensed); Community ISD example (`communityisd.org`).";

const TX_ARM_BASIS: &str = "`coach_contacts` is the registry's Texas coach-mailbox surface and the plan already refuses it as an engineering gap; the held UIL alignments, prototype rows and school pages carry coach names without mailboxes, so the lane records the work order's true-missing set instead of arming";

pub(super) const POLICY: Lane = Lane {
    jurisdiction: UsJurisdiction::Texas,
    arm: TX_ARM,
    refusal: (TX_REFUSAL_SLUG, TX_TRUE_MISSING),
    kind: RefusalKind::EngineeringGap,
    basis: ArmBasis::Text(TX_ARM_BASIS),
    policy: LanePolicy::Blocked,
};

#[cfg(test)]
mod tests {
    use super::super::bind;
    use super::*;
    use crate::restate_services::jurisdiction::{coach_lane_owed, owed_work};
    use crate::restate_services::plan::{plan as planned, BrowserLaneState};
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
            jurisdiction: UsJurisdiction::Texas,
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
            &planned(UsJurisdiction::Texas, BrowserLaneState::Absent),
            "fixture".to_string(),
        );
        let obligation = bind(&mut plan, UsJurisdiction::Texas, season()?, 1, OBSERVED_ON)?
            .ok_or("the Texas lane is registered")?;
        Ok((plan, obligation))
    }

    #[test]
    fn coach_lane_tx_records_the_true_missing_refusal_without_closing() -> TestResult {
        let (plan, recorded) = bound()?;
        check!(eq; recorded.id(), "coach_lane:TX:2026:1");
        check!(eq; recorded.status(), CoachLaneStatus::Blocked);
        check!(!recorded.is_terminal());
        check!(eq; recorded.arm_name(), TX_ARM);
        check!(recorded.evidence().is_none());
        check!(recorded.acceptance().is_none());
        let refusal = recorded
            .refusal()
            .ok_or("a recorded refusal carries its source and reason")?;
        check!(eq; refusal.source().id.as_str(), TX_REFUSAL_SLUG);
        check!(eq; refusal.reason(), TX_TRUE_MISSING);
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
            .any(|reason| reason.contains(TX_REFUSAL_SLUG)));
        check!(reasons
            .iter()
            .any(|reason| reason.contains(TX_TRUE_MISSING)));
        Ok(())
    }

    #[test]
    fn coach_lane_tx_true_missing_reason_is_the_work_order_line() -> TestResult {
        check!(WORK_ORDER.contains(TX_TRUE_MISSING));
        check!(WORK_ORDER.contains("TEA AskTED personnel downloads"));
        Ok(())
    }

    #[test]
    fn coach_lane_tx_refusal_is_recorded_in_the_durable_plan_path() -> TestResult {
        let (plan, recorded) = bound()?;
        check!(plan.has_engineering_gaps());
        check!(plan
            .refused
            .iter()
            .any(|entry| entry.slug == TX_REFUSAL_SLUG && entry.reason == TX_TRUE_MISSING));
        let refusals = plan.coach_lane_refusals(OBSERVED_ON)?;
        check!(refusals
            .iter()
            .any(|refusal| refusal.source().id == TX_REFUSAL_SLUG
                && refusal.reason() == TX_TRUE_MISSING));
        check!(eq; recorded.arm_name(), TX_ARM);
        Ok(())
    }

    #[test]
    fn coach_lane_tx_arm_is_not_claimed_sweepable_by_an_arming_lane() -> TestResult {
        let obligation = super::super::evaluate(&POLICY, season()?, 1, OBSERVED_ON)?;
        check!(eq; obligation.arm_name(), TX_ARM);
        check!(!obligation.is_terminal());
        check!(obligation.evidence().is_none());
        Ok(())
    }

    #[test]
    fn coach_lane_tx_refusal_survives_reopen_and_replay_without_acceptance() -> TestResult {
        use census_store::{Store, Table};

        let root = tempfile::tempdir()?;
        let (plan, obligation) = bound()?;
        let mut state = JurisdictionState {
            identity: "jurisdiction:TX:2026-27:1".to_string(),
            plan: Some(plan),
            coach_lane: Some(obligation.clone()),
            ..JurisdictionState::default()
        };
        let expected = serde_json::to_value(&state)?;
        let phase = "tx-coach-refusal-qualification";
        {
            let store = Store::open(root.path())?;
            persist_refusal(&store, &state, phase)?;
        }
        {
            let store = Store::open(root.path())?;
            state = serde_json::from_value(
                store
                    .journal_payload(phase, &state.identity)?
                    .ok_or("TX state missing")?,
            )?;
            check!(eq; store.scan::<CoachLaneObligation>(Table::CoachLanes)?, vec![obligation]);
            for _ in 0..3 {
                state.coach_lane = bind(
                    state.plan.as_mut().ok_or("TX plan missing")?,
                    UsJurisdiction::Texas,
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
        use crate::restate_services::plan::RefusalKind;

        let row = state.coach_lane.as_ref().ok_or("TX row missing")?;
        check!(eq; row.status(), CoachLaneStatus::Blocked);
        check!(row.acceptance().is_none());
        check!(row.evidence().is_none());
        check!(coach_lane_owed(state));
        let entries: Vec<_> = state
            .plan
            .as_ref()
            .ok_or("TX plan missing")?
            .refused
            .iter()
            .filter(|entry| entry.slug == TX_REFUSAL_SLUG)
            .collect();
        check!(eq; entries.len(), 1);
        let entry = entries.first().ok_or("TX refusal missing")?;
        check!(eq; entry.reason.as_str(), TX_TRUE_MISSING);
        check!(eq; entry.kind, RefusalKind::EngineeringGap);
        store.replace(census_store::Table::CoachLanes, row)?;
        store.journal_done(phase, &state.identity, state)?;
        store.flush()?;
        Ok(())
    }
}
