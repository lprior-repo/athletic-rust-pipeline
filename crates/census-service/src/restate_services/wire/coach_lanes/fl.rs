use census_domain::model::{
    CoachLaneError, CoachLaneObligation, CoachLaneRefusal, CoachLaneResolution, CoachLaneStage,
    PendingCoachLane, SchoolYear, SourceRef,
};
use census_domain::UsJurisdiction;

pub(super) const FL_ARM: &str = "home_campus";
pub(super) const FL_REFUSAL_SLUG: &str = "fl_missing_coach_sources";
pub(super) const FL_TRUE_MISSING: &str = "FHSAA XC/TF classification news files (article not captured); FACA (`floridacoaches.org`); SSAA (`sunshinestateathletics`); FICAA (`ficaa.org`); FCC (`fccsports.net`); FACCS (`faccs.org`); FCIS (`fcis.org`); Cypress Creek HS example (`cchs.pasco`); Winter Springs HS (`winterspringshs`).";
pub(super) const FL_CAPTURE_PATH: &str = "var/home-campus-CA-20261004/fl.log";
pub(super) const FL_CAPTURE_DIGEST: &str =
    "34d59e4eb1529b93a64be617e3d84847838c61d9ae86fe2b4a89bbedba0129b9";
#[cfg(test)]
const FL_CAPTURE_FILE: &str =
    "crates/census-crawl/tests/fixtures/coach_lane_captures/home-campus-CA-20261004/fl.log";

const FL_ARM_BASIS: &str = "the wired home_campus arm parses FHSAA school profiles, but the retained 2026-10-04 fl.log is an aggregate report of 880 schools and 3677 email rows, not page-bound role, current-tenure or source-origin evidence, so it cannot arm the lane:";

pub(super) fn lane(
    season: SchoolYear,
    revision: u32,
    observed_on: &str,
) -> Result<CoachLaneObligation, CoachLaneError> {
    let pending = CoachLaneObligation::pending(PendingCoachLane {
        jurisdiction: UsJurisdiction::Florida,
        season,
        revision,
        arm: FL_ARM.to_string(),
        arm_basis: format!("{FL_ARM_BASIS} {FL_CAPTURE_PATH} sha256 {FL_CAPTURE_DIGEST}"),
        observed_on: observed_on.to_string(),
    })?;
    pending.record(
        CoachLaneStage::Blocked,
        Some(CoachLaneRefusal::new(
            SourceRef::id(FL_REFUSAL_SLUG),
            FL_TRUE_MISSING,
            observed_on,
        )?),
        CoachLaneResolution::unresolved(observed_on),
    )
}

#[cfg(test)]
mod tests {
    use super::super::{bind, capture_digest};
    use super::*;
    use crate::restate_services::jurisdiction::coach_lane_owed;
    use crate::restate_services::plan::{plan as planned, BrowserLaneState};
    use crate::restate_services::teams_arms;
    use crate::restate_services::wire::{JurisdictionState, SourcePlan};
    use census_domain::model::CoachLaneStatus;
    use sha2::{Digest, Sha256};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const OBSERVED_ON: &str = "2026-10-11";
    const BOUNDARY_ON: &str = "2027-11-08";
    const STALE_ON: &str = "2027-11-09";
    const CAPTURE_BYTES: usize = 294;
    const WORK_ORDER: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/sources/coach-coverage-bundle-20261004/state-work-orders/group-a.md"
    ));

    fn season() -> Result<SchoolYear, Box<dyn std::error::Error>> {
        Ok(SchoolYear::new(2026).ok_or("school year 2026")?)
    }

    fn bound() -> Result<(SourcePlan, CoachLaneObligation), Box<dyn std::error::Error>> {
        let mut plan = SourcePlan::of(
            &planned(UsJurisdiction::Florida, BrowserLaneState::Absent),
            "fixture".to_string(),
        );
        let obligation = bind(
            &mut plan,
            UsJurisdiction::Florida,
            season()?,
            1,
            OBSERVED_ON,
        )?
        .ok_or("the Florida lane is registered")?;
        Ok((plan, obligation))
    }

    #[test]
    fn coach_lane_fl_records_the_log_only_refusal_without_closing() -> TestResult {
        let (plan, recorded) = bound()?;
        check!(eq; recorded.id(), "coach_lane:FL:2026:1");
        check!(eq; recorded.status(), CoachLaneStatus::Blocked);
        check!(!recorded.is_terminal());
        check!(eq; recorded.arm_name(), FL_ARM);
        check!(recorded.evidence().is_none());
        check!(recorded.acceptance().is_none());
        let refusal = recorded
            .refusal()
            .ok_or("log-only evidence remains refused")?;
        check!(eq; refusal.source().id.as_str(), FL_REFUSAL_SLUG);
        check!(eq; refusal.reason(), FL_TRUE_MISSING);
        check!(recorded.arm_basis().contains(FL_CAPTURE_PATH));
        check!(recorded.arm_basis().contains(FL_CAPTURE_DIGEST));
        let state = JurisdictionState {
            plan: Some(plan),
            coach_lane: Some(recorded),
            ..JurisdictionState::default()
        };
        check!(coach_lane_owed(&state));
        Ok(())
    }

    #[test]
    fn coach_lane_fl_evidence_digest_is_the_sha256_of_the_capture() -> TestResult {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let bytes = std::fs::read(root.join(FL_CAPTURE_FILE))?;
        check!(eq; bytes.len(), CAPTURE_BYTES);
        check!(eq; format!("{:x}", Sha256::digest(&bytes)).as_str(), FL_CAPTURE_DIGEST);
        check!(eq; capture_digest(FL_CAPTURE_FILE)?, FL_CAPTURE_DIGEST);
        Ok(())
    }

    #[test]
    fn coach_lane_fl_arm_is_wired_into_the_plan_and_the_engine_arm_table() -> TestResult {
        let (plan, armed) = bound()?;
        check!(plan.sweepable.iter().any(|slug| slug == FL_ARM));
        check!(teams_arms::arm_for(FL_ARM).is_some());
        check!(eq; armed.status(), CoachLaneStatus::Blocked);
        Ok(())
    }

    #[test]
    fn coach_lane_fl_log_age_cannot_supply_missing_qualification_evidence() -> TestResult {
        for observed_on in [OBSERVED_ON, BOUNDARY_ON, STALE_ON] {
            let recorded = lane(season()?, 1, observed_on)?;
            check!(eq; recorded.status(), CoachLaneStatus::Blocked);
            check!(!recorded.is_terminal());
            check!(recorded.evidence().is_none());
            let refusal = recorded
                .refusal()
                .ok_or("log-only evidence remains refused")?;
            check!(eq; refusal.reason(), FL_TRUE_MISSING);
        }
        Ok(())
    }

    #[test]
    fn coach_lane_fl_true_missing_reason_is_the_work_order_line() -> TestResult {
        check!(WORK_ORDER.contains(FL_TRUE_MISSING));
        check!(WORK_ORDER.contains("FHSAA XC/TF classification news files"));
        Ok(())
    }

    #[test]
    fn coach_lane_fl_refusal_is_recorded_in_the_durable_plan_path() -> TestResult {
        let (plan, _) = bound()?;
        check!(plan
            .refused
            .iter()
            .any(|entry| entry.slug == FL_REFUSAL_SLUG && entry.reason == FL_TRUE_MISSING));
        let refusals = plan.coach_lane_refusals(OBSERVED_ON)?;
        check!(refusals
            .iter()
            .any(|refusal| refusal.source().id == FL_REFUSAL_SLUG
                && refusal.reason() == FL_TRUE_MISSING));
        let unarmed = JurisdictionState {
            plan: Some(plan),
            ..JurisdictionState::default()
        };
        check!(coach_lane_owed(&unarmed));
        Ok(())
    }
}
