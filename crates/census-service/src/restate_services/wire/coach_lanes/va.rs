#[cfg(test)]
use census_domain::model::{CoachLaneObligation, SchoolYear};
use census_domain::UsJurisdiction;

use super::policy::{ArmBasis, Lane, LanePolicy};
use crate::restate_services::plan::RefusalKind;

pub(super) const VA_ARM: &str = "milesplit";
pub(super) const VA_REFUSAL_SLUG: &str = "va_missing_coach_sources";
pub(super) const VA_TRUE_MISSING: &str = "VHSL member directory (`vhsl.org`); VHSL Reports (`vhslreports.com`); VISAA XC/TF participation docs; VCPE (`vcpe.org`); Virginia Metro Athletic Conference; Virginia Track Coaches Association (`runsignup.com` MemberOrg).";

const VA_ARM_BASIS: &str = "the wired milesplit arm is the only Virginia surface the plan sweeps with names and the held VDOE and VISAA bodies are school lists only while the VHSL qualified.json body is partial, so the lane records the work order's true-missing set instead of arming";

pub(super) const POLICY: Lane = Lane {
    jurisdiction: UsJurisdiction::Virginia,
    arm: VA_ARM,
    refusal: (VA_REFUSAL_SLUG, VA_TRUE_MISSING),
    kind: RefusalKind::EngineeringGap,
    basis: ArmBasis::Text(VA_ARM_BASIS),
    policy: LanePolicy::Blocked,
};

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
            jurisdiction: UsJurisdiction::Virginia,
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
            &planned(UsJurisdiction::Virginia, BrowserLaneState::Absent),
            "fixture".to_string(),
        );
        let obligation = bind(
            &mut plan,
            UsJurisdiction::Virginia,
            season()?,
            1,
            OBSERVED_ON,
        )?
        .ok_or("the Virginia lane is registered")?;
        Ok((plan, obligation))
    }

    #[test]
    fn coach_lane_va_records_the_true_missing_refusal_without_closing() -> TestResult {
        let (plan, recorded) = bound()?;
        check!(eq; recorded.id(), "coach_lane:VA:2026:1");
        check!(eq; recorded.status(), CoachLaneStatus::Blocked);
        check!(!recorded.is_terminal());
        check!(eq; recorded.arm_name(), VA_ARM);
        check!(recorded.evidence().is_none());
        check!(recorded.acceptance().is_none());
        let refusal = recorded
            .refusal()
            .ok_or("a recorded refusal carries its source and reason")?;
        check!(eq; refusal.source().id.as_str(), VA_REFUSAL_SLUG);
        check!(eq; refusal.reason(), VA_TRUE_MISSING);
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
            .any(|reason| reason.contains(VA_REFUSAL_SLUG)));
        check!(reasons
            .iter()
            .any(|reason| reason.contains(VA_TRUE_MISSING)));
        Ok(())
    }

    #[test]
    fn coach_lane_va_true_missing_reason_is_the_work_order_line() -> TestResult {
        check!(WORK_ORDER.contains(VA_TRUE_MISSING));
        check!(WORK_ORDER.contains("VHSL member directory"));
        Ok(())
    }

    #[test]
    fn coach_lane_va_refusal_is_recorded_in_the_durable_plan_path() -> TestResult {
        let (plan, recorded) = bound()?;
        check!(plan.has_engineering_gaps());
        check!(plan
            .refused
            .iter()
            .any(|entry| entry.slug == VA_REFUSAL_SLUG && entry.reason == VA_TRUE_MISSING));
        let refusals = plan.coach_lane_refusals(OBSERVED_ON)?;
        check!(refusals
            .iter()
            .any(|refusal| refusal.source().id == VA_REFUSAL_SLUG
                && refusal.reason() == VA_TRUE_MISSING));
        check!(eq; recorded.arm_name(), VA_ARM);
        Ok(())
    }

    #[test]
    fn coach_lane_va_arm_is_the_wired_milesplit_source() -> TestResult {
        let (plan, _) = bound()?;
        check!(teams_arms::arm_for(VA_ARM).is_some());
        check!(plan.sweepable.iter().any(|slug| slug == VA_ARM));
        Ok(())
    }

    #[test]
    fn coach_lane_va_refusal_survives_store_reopen() -> TestResult {
        let root = tempfile::tempdir()?;
        let (plan, recorded) = bound()?;
        let state = JurisdictionState {
            plan: Some(plan),
            coach_lane: Some(recorded.clone()),
            ..JurisdictionState::default()
        };
        let store = census_store::Store::open(root.path())?;
        store.replace(census_store::Table::CoachLanes, &recorded)?;
        store.journal_done("va_refusal_durability", recorded.id(), &state)?;
        drop(store);
        let reopened = census_store::Store::open(root.path())?;
        let rows: Vec<CoachLaneObligation> = reopened.scan(census_store::Table::CoachLanes)?;
        check!(eq; rows, vec![recorded.clone()]);
        let payload = reopened
            .journal_payload("va_refusal_durability", recorded.id())?
            .ok_or("durable jurisdiction state")?;
        let restored: JurisdictionState = serde_json::from_value(payload)?;
        check!(coach_lane_owed(&restored));
        let (owed, reasons) = owed_work(&restored, &request()?);
        check!(owed.iter().any(|stage| stage == "coach_lane"));
        check!(reasons
            .iter()
            .any(|reason| reason.contains(VA_TRUE_MISSING)));
        check!(restored
            .coach_lane
            .ok_or("restored lane")?
            .acceptance()
            .is_none());
        Ok(())
    }
}
