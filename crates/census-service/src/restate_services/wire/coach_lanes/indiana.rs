#[cfg(test)]
use census_domain::model::{CoachLaneObligation, SchoolYear};
use census_domain::UsJurisdiction;

use super::policy::{ArmBasis, Lane, LanePolicy};
use crate::restate_services::plan::RefusalKind;

pub(super) const IN_ARM: &str = "coach_contacts";
pub(super) const IN_REFUSAL_SLUG: &str = "in_missing_coach_sources";
pub(super) const IN_TRUE_MISSING: &str = "IHSAA per-school detail pages; Indiana Association of Christian Schools (`indianaacs.org`); Eventlink XC (`eventlink.com`); Hoosier Crossroads Conference; school coach pages.";

const IN_ARM_BASIS: &str = "`coach_contacts` is the registry's Indiana coach-mailbox surface and the plan refuses it as an engineering gap; the held IHSAA extract is a header-only 108 B rows.csv, the DOE extract is a school roster without coach columns and the IATCCC tap retained robots only, so the lane records the work order's true-missing set instead of arming";

pub(super) const POLICY: Lane = Lane {
    jurisdiction: UsJurisdiction::Indiana,
    arm: IN_ARM,
    refusal: (IN_REFUSAL_SLUG, IN_TRUE_MISSING),
    kind: RefusalKind::EngineeringGap,
    basis: ArmBasis::Text(IN_ARM_BASIS),
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
            jurisdiction: UsJurisdiction::Indiana,
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
            &planned(UsJurisdiction::Indiana, BrowserLaneState::Absent),
            "fixture".to_string(),
        );
        let obligation = bind(
            &mut plan,
            UsJurisdiction::Indiana,
            season()?,
            1,
            OBSERVED_ON,
        )?
        .ok_or("the Indiana lane is registered")?;
        Ok((plan, obligation))
    }

    #[test]
    fn coach_lane_in_records_the_true_missing_refusal_without_closing() -> TestResult {
        let (plan, recorded) = bound()?;
        check!(eq; recorded.id(), "coach_lane:IN:2026:1");
        check!(eq; recorded.status(), CoachLaneStatus::Blocked);
        check!(!recorded.is_terminal());
        check!(eq; recorded.arm_name(), IN_ARM);
        check!(recorded.evidence().is_none());
        check!(recorded.acceptance().is_none());
        let refusal = recorded
            .refusal()
            .ok_or("a recorded refusal carries its source and reason")?;
        check!(eq; refusal.source().id.as_str(), IN_REFUSAL_SLUG);
        check!(eq; refusal.reason(), IN_TRUE_MISSING);
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
            .any(|reason| reason.contains(IN_REFUSAL_SLUG)));
        check!(reasons
            .iter()
            .any(|reason| reason.contains(IN_TRUE_MISSING)));
        Ok(())
    }

    #[test]
    fn coach_lane_in_true_missing_reason_is_the_work_order_line() -> TestResult {
        check!(WORK_ORDER.contains(IN_TRUE_MISSING));
        check!(WORK_ORDER.contains("IHSAA per-school detail pages"));
        Ok(())
    }

    #[test]
    fn coach_lane_in_refusal_is_recorded_in_the_durable_plan_path() -> TestResult {
        let (plan, recorded) = bound()?;
        check!(plan.has_engineering_gaps());
        check!(plan
            .refused
            .iter()
            .any(|entry| entry.slug == IN_REFUSAL_SLUG && entry.reason == IN_TRUE_MISSING));
        let refusals = plan.coach_lane_refusals(OBSERVED_ON)?;
        check!(refusals
            .iter()
            .any(|refusal| refusal.source().id == IN_REFUSAL_SLUG
                && refusal.reason() == IN_TRUE_MISSING));
        check!(eq; recorded.arm_name(), IN_ARM);
        Ok(())
    }

    #[test]
    fn coach_lane_in_arm_is_not_claimed_sweepable_by_an_arming_lane() -> TestResult {
        let (plan, recorded) = bound()?;
        check!(eq; recorded.arm_name(), IN_ARM);
        check!(!recorded.is_terminal());
        check!(recorded.evidence().is_none());
        check!(plan.refused.iter().any(|entry| entry.slug == IN_ARM));
        check!(!plan.sweepable.iter().any(|slug| slug == IN_ARM));
        Ok(())
    }

    #[test]
    fn coach_lane_in_refusal_survives_store_reopen_and_rebind() -> TestResult {
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
            store.journal_done("in-coach-qualification", &state.identity, &state)?;
            store.flush()?;
        }
        let store = Store::open(root.path())?;
        check!(eq; store.scan::<CoachLaneObligation>(Table::CoachLanes)?, vec![recorded]);
        let payload = store
            .journal_payload("in-coach-qualification", &state.identity)?
            .ok_or("Indiana refusal lost on reopen")?;
        let mut retained: JurisdictionState = serde_json::from_value(payload)?;
        check!(eq; serde_json::to_value(&retained)?, serde_json::to_value(&state)?);
        let plan = retained.plan.as_mut().ok_or("Indiana plan missing")?;
        retained.coach_lane = bind(plan, UsJurisdiction::Indiana, season()?, 1, OBSERVED_ON)?;
        check!(eq; serde_json::to_value(&retained)?, serde_json::to_value(&state)?);
        check!(coach_lane_owed(&retained));
        let refusal = retained
            .coach_lane
            .as_ref()
            .and_then(CoachLaneObligation::refusal)
            .ok_or("Indiana refusal missing")?;
        check!(eq; refusal.reason(), IN_TRUE_MISSING);
        check!(eq; refusal.source().id.as_str(), IN_REFUSAL_SLUG);
        check!(eq; store.journal_keys("in-coach-qualification")?.len(), 1);
        check!(eq; store.walk_table(Table::Coaches)?.rows, 0);
        Ok(())
    }
}
