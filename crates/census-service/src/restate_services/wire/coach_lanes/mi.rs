use census_domain::model::{
    CoachLaneError, CoachLaneObligation, CoachLaneRefusal, CoachLaneResolution, CoachLaneStage,
    PendingCoachLane, SchoolYear, SourceRef,
};
use census_domain::UsJurisdiction;

pub(super) const MI_ARM: &str = "coach_contacts";
pub(super) const MI_REFUSAL_SLUG: &str = "mi_missing_coach_sources";
pub(super) const MI_TRUE_MISSING: &str = "`my.mhsaa.com` leagues search; MHSCA (`mhsca.org`); CHSL (`chsl.com`); MI approved nonpublic list (`michigan.gov`); school athletics pages.";

const MI_ARM_BASIS: &str = "`coach_contacts` is the registry's Michigan coach-mailbox surface and the plan refuses it as an engineering gap; the held MHSAA bodies are school-level coaching-tab HTML, the MITCA probe rows are name-only and no retained Michigan body carries per-school coach mailboxes, so the lane records the work order's true-missing set instead of arming";

pub(super) fn lane(
    season: SchoolYear,
    revision: u32,
    observed_on: &str,
) -> Result<CoachLaneObligation, CoachLaneError> {
    let pending = CoachLaneObligation::pending(PendingCoachLane {
        jurisdiction: UsJurisdiction::Michigan,
        season,
        revision,
        arm: MI_ARM.to_string(),
        arm_basis: MI_ARM_BASIS.to_string(),
        observed_on: observed_on.to_string(),
    })?;
    pending.record(
        CoachLaneStage::Blocked,
        Some(refusal(observed_on)?),
        CoachLaneResolution::unresolved(observed_on),
    )
}

pub(super) fn refusal(observed_on: &str) -> Result<CoachLaneRefusal, CoachLaneError> {
    CoachLaneRefusal::new(SourceRef::id(MI_REFUSAL_SLUG), MI_TRUE_MISSING, observed_on)
}

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
            jurisdiction: UsJurisdiction::Michigan,
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
            &planned(UsJurisdiction::Michigan, BrowserLaneState::Absent),
            "fixture".to_string(),
        );
        let obligation = bind(
            &mut plan,
            UsJurisdiction::Michigan,
            season()?,
            1,
            OBSERVED_ON,
        )?
        .ok_or("the Michigan lane is registered")?;
        Ok((plan, obligation))
    }

    #[test]
    fn coach_lane_mi_records_the_true_missing_refusal_without_closing() -> TestResult {
        let (plan, recorded) = bound()?;
        check!(eq; recorded.id(), "coach_lane:MI:2026:1");
        check!(eq; recorded.status(), CoachLaneStatus::Blocked);
        check!(!recorded.is_terminal());
        check!(eq; recorded.arm_name(), MI_ARM);
        check!(recorded.evidence().is_none());
        check!(recorded.acceptance().is_none());
        let refusal = recorded
            .refusal()
            .ok_or("a recorded refusal carries its source and reason")?;
        check!(eq; refusal.source().id.as_str(), MI_REFUSAL_SLUG);
        check!(eq; refusal.reason(), MI_TRUE_MISSING);
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
            .any(|reason| reason.contains(MI_REFUSAL_SLUG)));
        check!(reasons
            .iter()
            .any(|reason| reason.contains(MI_TRUE_MISSING)));
        Ok(())
    }

    #[test]
    fn coach_lane_mi_true_missing_reason_is_the_work_order_line() -> TestResult {
        check!(WORK_ORDER.contains(MI_TRUE_MISSING));
        check!(WORK_ORDER.contains("`my.mhsaa.com` leagues search"));
        Ok(())
    }

    #[test]
    fn coach_lane_mi_refusal_is_recorded_in_the_durable_plan_path() -> TestResult {
        let (plan, recorded) = bound()?;
        check!(plan.has_engineering_gaps());
        check!(plan
            .refused
            .iter()
            .any(|entry| entry.slug == MI_REFUSAL_SLUG && entry.reason == MI_TRUE_MISSING));
        let refusals = plan.coach_lane_refusals(OBSERVED_ON)?;
        check!(refusals
            .iter()
            .any(|refusal| refusal.source().id == MI_REFUSAL_SLUG
                && refusal.reason() == MI_TRUE_MISSING));
        check!(eq; recorded.arm_name(), MI_ARM);
        Ok(())
    }

    #[test]
    fn coach_lane_mi_arm_is_not_claimed_sweepable_by_an_arming_lane() -> TestResult {
        let (plan, recorded) = bound()?;
        check!(eq; recorded.arm_name(), MI_ARM);
        check!(!recorded.is_terminal());
        check!(recorded.evidence().is_none());
        check!(plan.refused.iter().any(|entry| entry.slug == MI_ARM));
        check!(!plan.sweepable.iter().any(|slug| slug == MI_ARM));
        Ok(())
    }

    #[test]
    fn coach_lane_mi_refusal_survives_store_reopen() -> TestResult {
        let root = tempfile::tempdir()?;
        let (plan, recorded) = bound()?;
        let state = JurisdictionState {
            plan: Some(plan),
            coach_lane: Some(recorded.clone()),
            ..JurisdictionState::default()
        };
        let store = census_store::Store::open(root.path())?;
        store.replace(census_store::Table::CoachLanes, &recorded)?;
        store.journal_done("mi_refusal_durability", recorded.id(), &state)?;
        drop(store);
        let reopened = census_store::Store::open(root.path())?;
        let rows: Vec<CoachLaneObligation> = reopened.scan(census_store::Table::CoachLanes)?;
        check!(eq; rows, vec![recorded.clone()]);
        let payload = reopened
            .journal_payload("mi_refusal_durability", recorded.id())?
            .ok_or("durable jurisdiction state")?;
        let restored: JurisdictionState = serde_json::from_value(payload)?;
        check!(coach_lane_owed(&restored));
        let (owed, reasons) = owed_work(&restored, &request()?);
        check!(owed.iter().any(|stage| stage == "coach_lane"));
        check!(reasons
            .iter()
            .any(|reason| reason.contains(MI_TRUE_MISSING)));
        check!(restored
            .coach_lane
            .ok_or("restored lane")?
            .acceptance()
            .is_none());
        Ok(())
    }
}
