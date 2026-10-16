use census_domain::model::{
    CoachLaneError, CoachLaneEvidence, CoachLaneMailbox, CoachLaneObligation, CoachLaneRefusal,
    CoachLaneResolution, CoachLaneRole, CoachLaneStage, CoachLaneTenure, PendingCoachLane,
    SchoolYear, SourceRef, COACH_LANE_MAX_FRESHNESS_DAYS,
};
use census_domain::UsJurisdiction;

use super::freshness_days;

pub(super) const CO_ARM: &str = "chsaa";
pub(super) const CO_REFUSAL_SLUG: &str = "co_missing_coach_sources";
pub(super) const CO_TRUE_MISSING: &str = "CHSAA per-school detail pages beyond the 3 samples (`chsaanow.com/schools/*`); Colorado High School Coaches Association (CHSCA); Colorado TF/XC coaches association; school athletics staff pages.";
pub(super) const CO_DIRECTORY_PATH: &str =
    "crates/census-crawl/tests/fixtures/chsaa/directory.html";
pub(super) const CO_DIRECTORY_DIGEST: &str =
    "5b6fcae4a8ac9844ba4da74cf23512b00beb1ec30a6dd5078a7c263f229778f9";
pub(super) const CO_CHERRY_PATH: &str =
    "crates/census-crawl/tests/fixtures/chsaa/school_cherry_creek.html";
pub(super) const CO_CHERRY_DIGEST: &str =
    "784e16bbabec8e7dfc373b9f8dbb601eb4d281bd3f60310f5362312c59db90da";
pub(super) const CO_ROWS_PATH: &str =
    "crates/census-crawl/tests/fixtures/chsaa/golden_school_coach_rows.json";
pub(super) const CO_ROWS_DIGEST: &str =
    "cc7a4e23ea461f65a20a448754781cce17bfc05e79ee351abe2630f76516e5a4";
pub(super) const CO_CAPTURES: [(&str, &str); 3] = [
    (CO_DIRECTORY_PATH, CO_DIRECTORY_DIGEST),
    (CO_CHERRY_PATH, CO_CHERRY_DIGEST),
    (CO_ROWS_PATH, CO_ROWS_DIGEST),
];
pub(super) const CO_SOURCE_URL: &str = "https://chsaanow.com/schools/";
pub(super) const CO_SCHOOLS: u32 = 378;
pub(super) const CO_CONTACTS: u32 = 50;
pub(super) const CO_CAPTURE_ON: &str = "2026-09-27";

const CO_ARM_BASIS: &str = "the chsaa descriptor declares school-coach contact, is admitted for Colorado only, and TeamsArm::ChsaaSchools walks the member directory and each school page; the retained CHSAA captures are the held output of that walk";
const CO_RESOLUTION_NOTE: &str = "the retained CHSAA 2026-09-27 captures: the member directory holds 378 schools with street addresses; the Cherry Creek school page carries 50 coach rows with Head Coach and Assistant Coach titles including Boys Cross Country staff; neither held body carries a coach mailbox, so mailbox coverage remains the recorded true-missing work and the other 377 school pages are not part of this evidence set:";

fn resolved_note() -> String {
    let mut note = String::from(CO_RESOLUTION_NOTE);
    for (path, digest) in CO_CAPTURES {
        note.push_str(&format!(" {path} sha256 {digest};"));
    }
    note
}

pub(super) fn lane(
    season: SchoolYear,
    revision: u32,
    observed_on: &str,
) -> Result<CoachLaneObligation, CoachLaneError> {
    let pending = CoachLaneObligation::pending(PendingCoachLane {
        jurisdiction: UsJurisdiction::Colorado,
        season,
        revision,
        arm: CO_ARM.to_string(),
        arm_basis: CO_ARM_BASIS.to_string(),
        observed_on: observed_on.to_string(),
    })?;
    let freshness = freshness_days(CO_CAPTURE_ON, observed_on)?;
    if freshness > COACH_LANE_MAX_FRESHNESS_DAYS {
        return pending.record(
            CoachLaneStage::Blocked,
            Some(stale_refusal(freshness, observed_on)?),
            CoachLaneResolution::unresolved(observed_on),
        );
    }
    let evidence =
        CoachLaneEvidence::new(CO_SCHOOLS, CO_CONTACTS, CO_DIRECTORY_DIGEST, CO_CAPTURE_ON)?
            .with_source_url(CO_SOURCE_URL)?;
    let resolution = CoachLaneResolution::resolved(
        CoachLaneRole::Published,
        CoachLaneTenure::Current,
        CoachLaneMailbox::Unpublished,
        CO_CAPTURE_ON,
        freshness,
        resolved_note(),
    )?;
    pending.arm(evidence, resolution)
}

fn stale_refusal(freshness: u32, observed_on: &str) -> Result<CoachLaneRefusal, CoachLaneError> {
    CoachLaneRefusal::new(
        SourceRef::id(CO_ARM),
        format!(
            "the held chsaa capture {CO_DIRECTORY_PATH} is {freshness} days old at lane creation, beyond the {COACH_LANE_MAX_FRESHNESS_DAYS}-day bound, so the Colorado arm cannot retain it"
        ),
        observed_on,
    )
}

#[cfg(test)]
mod tests {
    use super::super::{bind, freshness_days, LANES};
    use super::*;
    use crate::restate_services::jurisdiction::coach_lane_owed;
    use crate::restate_services::plan::{plan as planned, BrowserLaneState};
    use crate::restate_services::teams_arms;
    use crate::restate_services::wire::{JurisdictionState, SourcePlan};
    use census_domain::model::CoachLaneStatus;
    use sha2::{Digest, Sha256};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const OBSERVED_ON: &str = "2026-10-11";
    const BOUNDARY_ON: &str = "2027-11-01";
    const STALE_ON: &str = "2027-11-02";
    const CO_CAPTURE_BYTES: [u64; 3] = [344_492, 351_964, 8_452];
    const PROVENANCE_JSON: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/census-crawl/tests/fixtures/chsaa/PROVENANCE.json"
    ));
    const GROUP_A: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/sources/coach-coverage-bundle-20261004/state-work-orders/group-a.md"
    ));

    fn season() -> Result<SchoolYear, Box<dyn std::error::Error>> {
        Ok(SchoolYear::new(2026).ok_or("school year 2026")?)
    }

    fn bound() -> Result<(SourcePlan, CoachLaneObligation), Box<dyn std::error::Error>> {
        let mut plan = SourcePlan::of(
            &planned(UsJurisdiction::Colorado, BrowserLaneState::Absent),
            "fixture".to_string(),
        );
        let obligation = bind(
            &mut plan,
            UsJurisdiction::Colorado,
            season()?,
            1,
            OBSERVED_ON,
        )?
        .ok_or("the Colorado lane is registered")?;
        Ok((plan, obligation))
    }

    #[test]
    fn coach_lane_co_arms_on_the_retained_chsaa_captures() -> TestResult {
        let (plan, armed) = bound()?;
        check!(eq; armed.id(), "coach_lane:CO:2026:1");
        check!(eq; armed.status(), CoachLaneStatus::Armed);
        check!(armed.is_terminal());
        check!(eq; armed.arm_name(), CO_ARM);
        check!(armed.refusal().is_none());
        check!(armed.acceptance().is_none());
        let evidence = armed.evidence().ok_or("an armed lane carries evidence")?;
        check!(eq; evidence.schools(), CO_SCHOOLS);
        check!(eq; evidence.contacts(), CO_CONTACTS);
        check!(eq; evidence.digest(), CO_DIRECTORY_DIGEST);
        check!(eq; evidence.observed_on(), CO_CAPTURE_ON);
        check!(eq; evidence.source_url(), Some(CO_SOURCE_URL));
        let resolution = armed.resolution();
        check!(eq; resolution.role, CoachLaneRole::Published);
        check!(eq; resolution.tenure, CoachLaneTenure::Current);
        check!(eq; resolution.mailbox, CoachLaneMailbox::Unpublished);
        check!(eq; resolution.retrieved_on, CO_CAPTURE_ON);
        check!(eq; resolution.freshness_days, 14);
        check!(resolution.freshness_days <= COACH_LANE_MAX_FRESHNESS_DAYS);
        let note = resolution
            .evidence
            .as_deref()
            .ok_or("a resolved lane carries a note")?;
        for (path, digest) in CO_CAPTURES {
            check!(note.contains(path));
            check!(note.contains(digest));
        }
        let mut state = JurisdictionState {
            plan: Some(plan),
            ..JurisdictionState::default()
        };
        check!(coach_lane_owed(&state));
        state.coach_lane = Some(armed);
        check!(!coach_lane_owed(&state));
        Ok(())
    }

    #[test]
    fn coach_lane_co_capture_digests_match_the_held_fixtures() -> TestResult {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for ((path, digest), bytes) in CO_CAPTURES.into_iter().zip(CO_CAPTURE_BYTES) {
            let file = path
                .rsplit('/')
                .next()
                .ok_or("a capture path carries a file name")?;
            let body = std::fs::read(root.join(path))?;
            check!(eq; body.len() as u64, bytes, "{file} byte count");
            check!(eq; format!("{:x}", Sha256::digest(&body)), digest, "{file} sha256");
            check!(
                PROVENANCE_JSON.contains(file),
                "PROVENANCE.json omits {file}"
            );
        }
        // The provenance pins shas for the two raw captures; the golden rows
        // carry no stored sha, so only the raw pair is cross-checked here.
        for digest in [CO_DIRECTORY_DIGEST, CO_CHERRY_DIGEST] {
            check!(
                PROVENANCE_JSON.contains(digest),
                "PROVENANCE.json omits {digest}"
            );
        }
        Ok(())
    }

    #[test]
    fn coach_lane_co_arm_is_wired_into_the_plan_and_the_engine_arm_table() -> TestResult {
        let (plan, armed) = bound()?;
        check!(plan.sweepable.iter().any(|slug| slug == CO_ARM));
        check!(teams_arms::arm_for(CO_ARM).is_some());
        check!(eq; armed.status(), CoachLaneStatus::Armed);
        Ok(())
    }

    #[test]
    fn coach_lane_co_stale_capture_records_a_blocked_refusal_without_closing() -> TestResult {
        check!(eq; freshness_days(CO_CAPTURE_ON, BOUNDARY_ON)?, COACH_LANE_MAX_FRESHNESS_DAYS);
        check!(eq; freshness_days(CO_CAPTURE_ON, STALE_ON)?, COACH_LANE_MAX_FRESHNESS_DAYS + 1);
        let fresh = lane(season()?, 1, BOUNDARY_ON)?;
        check!(eq; fresh.status(), CoachLaneStatus::Armed);
        check!(eq; fresh.resolution().freshness_days, COACH_LANE_MAX_FRESHNESS_DAYS);
        let stale = lane(season()?, 1, STALE_ON)?;
        check!(eq; stale.status(), CoachLaneStatus::Blocked);
        check!(!stale.is_terminal());
        check!(stale.evidence().is_none());
        check!(stale.acceptance().is_none());
        let refusal = stale.refusal().ok_or("a stale capture records a refusal")?;
        check!(refusal.reason().contains("days old"));
        check!(refusal.reason().contains(CO_DIRECTORY_PATH));
        let state = JurisdictionState {
            coach_lane: Some(stale),
            ..JurisdictionState::default()
        };
        check!(coach_lane_owed(&state));
        Ok(())
    }

    #[test]
    fn coach_lane_co_remainder_matches_the_reviewed_true_missing() -> TestResult {
        check!(GROUP_A.contains(CO_TRUE_MISSING));
        Ok(())
    }

    #[test]
    fn coach_lane_co_refusal_is_recorded_in_the_durable_plan_path() -> TestResult {
        let (plan, armed) = bound()?;
        check!(plan
            .refused
            .iter()
            .any(|entry| entry.slug == CO_REFUSAL_SLUG && entry.reason == CO_TRUE_MISSING));
        let refusals = plan.coach_lane_refusals(OBSERVED_ON)?;
        check!(refusals
            .iter()
            .any(|refusal| refusal.source().id == CO_REFUSAL_SLUG
                && refusal.reason() == CO_TRUE_MISSING));
        check!(eq; armed.arm_name(), CO_ARM);
        check!(LANES
            .iter()
            .any(|row| row.jurisdiction == UsJurisdiction::Colorado));
        Ok(())
    }
}
