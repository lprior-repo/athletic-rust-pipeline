use census_domain::model::CoachLaneMailbox;
#[cfg(test)]
use census_domain::model::{
    CoachLaneObligation, CoachLaneRole, CoachLaneTenure, SchoolYear, COACH_LANE_MAX_FRESHNESS_DAYS,
};
use census_domain::UsJurisdiction;

use super::policy::{ArmBasis, Lane, LanePolicy, RetainedCapture};
use crate::restate_services::plan::RefusalKind;

pub(super) const IL_ARM: &str = "ihsa";
pub(super) const IL_REFUSAL_SLUG: &str = "il_missing_coach_sources";
pub(super) const IL_TRUE_MISSING: &str = "ITCCCA (`itccca.com`); Chicago Catholic League; ISBE nonpublic registration (`isbe.net`); IHSA conference route (`conf.htm` probe); school staff pages.";
pub(super) const IL_ABINGDON_PATH: &str =
    "research/sources/coach-directories-national/samples/validate/IL_Abingdon_Avon_0101_staff2.html";
pub(super) const IL_ABINGDON_DIGEST: &str =
    "e91d3c9f3073b20834c350bde8159cb15f69ae0f165e126d4ea8ca055ccbf2e4";
pub(super) const IL_DIXON_PATH: &str =
    "research/sources/coach-directories-national/samples/validate/IL_Dixon_0418_staff2.html";
pub(super) const IL_DIXON_DIGEST: &str =
    "7d65a70e940bc7141cbab4832b1e2d1f2188365fdcd6225e3cf7a31f0e1cd175";
pub(super) const IL_DOWNERS_PATH: &str =
    "research/sources/coach-directories-national/samples/validate/IL_Downers_Grove_North_0422_staff2.html";
pub(super) const IL_DOWNERS_DIGEST: &str =
    "5a4c102b2fde4b46bf0bf75fd8c52110c7a50087d6a871b0ab95d6d25244746e";
pub(super) const IL_CAPTURES: [(&str, &str); 3] = [
    (IL_ABINGDON_PATH, IL_ABINGDON_DIGEST),
    (IL_DIXON_PATH, IL_DIXON_DIGEST),
    (IL_DOWNERS_PATH, IL_DOWNERS_DIGEST),
];
pub(super) const IL_SOURCE_URL: &str = "https://api.ihsa.org/v1/schools";
pub(super) const IL_SCHOOLS: u32 = 3;
pub(super) const IL_CONTACTS: u32 = 3;
pub(super) const IL_CAPTURE_ON: &str = "2026-09-22";

const IL_ARM_BASIS: &str = "the ihsa descriptor declares school-coach contact, is admitted for Illinois only, and TeamsArm::IhsaSchools walks the member directory and each school's staff2; the retained IHSA captures are the held output of that walk";
const IL_RESOLUTION_NOTE: &str = "the retained IHSA 2026-09-22 staff2 captures: Abingdon-Avon 0101 publishes Boys XC Head Coach Mr. Barry Mink (mailbox bmink@atown276.net) with AD Mr. Reid Kelso; Dixon 0418 publishes Boys XC Head Coach Simon Thorpe (mailbox sthorpe@dps170.org) with AD Mr. Roger Fegan; Downers Grove North 0422 publishes 64 staff rows with 30 head-coach rows including Boys XC Head Coach John Sipple (mailbox jsipple@csd99.org) with AD Dave Rose; other member schools are not part of this evidence set:";

const CAPTURE: RetainedCapture = RetainedCapture {
    schools: IL_SCHOOLS,
    contacts: IL_CONTACTS,
    digest: IL_DOWNERS_DIGEST,
    captured_on: IL_CAPTURE_ON,
    source_url: IL_SOURCE_URL,
    mailbox: CoachLaneMailbox::Published,
    note: IL_RESOLUTION_NOTE,
    captures: &IL_CAPTURES,
    stale_path: IL_ABINGDON_PATH,
    jurisdiction_name: "Illinois",
};

pub(super) const POLICY: Lane = Lane {
    jurisdiction: UsJurisdiction::Illinois,
    arm: IL_ARM,
    refusal: (IL_REFUSAL_SLUG, IL_TRUE_MISSING),
    kind: RefusalKind::EngineeringGap,
    basis: ArmBasis::Text(IL_ARM_BASIS),
    policy: LanePolicy::RetainedCapture(&CAPTURE),
};

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
    const BOUNDARY_ON: &str = "2027-10-27";
    const STALE_ON: &str = "2027-10-28";
    const IL_CAPTURE_BYTES: [u64; 3] = [5_386, 6_394, 10_962];
    const COVERAGE_JSON: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/sources/coach-directories-national/coverage.json"
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
            &planned(UsJurisdiction::Illinois, BrowserLaneState::Absent),
            "fixture".to_string(),
        );
        let obligation = bind(
            &mut plan,
            UsJurisdiction::Illinois,
            season()?,
            1,
            OBSERVED_ON,
        )?
        .ok_or("the Illinois lane is registered")?;
        Ok((plan, obligation))
    }

    #[test]
    fn coach_lane_il_arms_on_the_retained_ihsa_captures() -> TestResult {
        let (plan, armed) = bound()?;
        check!(eq; armed.id(), "coach_lane:IL:2026:1");
        check!(eq; armed.status(), CoachLaneStatus::Armed);
        check!(armed.is_terminal());
        check!(eq; armed.arm_name(), IL_ARM);
        check!(armed.refusal().is_none());
        check!(armed.acceptance().is_none());
        let evidence = armed.evidence().ok_or("an armed lane carries evidence")?;
        check!(eq; evidence.schools(), IL_SCHOOLS);
        check!(eq; evidence.contacts(), IL_CONTACTS);
        check!(eq; evidence.digest(), IL_DOWNERS_DIGEST);
        check!(eq; evidence.observed_on(), IL_CAPTURE_ON);
        check!(eq; evidence.source_url(), Some(IL_SOURCE_URL));
        let resolution = armed.resolution();
        check!(eq; resolution.role, CoachLaneRole::Published);
        check!(eq; resolution.tenure, CoachLaneTenure::Current);
        check!(eq; resolution.mailbox, CoachLaneMailbox::Published);
        check!(eq; resolution.retrieved_on, IL_CAPTURE_ON);
        check!(eq; resolution.freshness_days, 19);
        check!(resolution.freshness_days <= COACH_LANE_MAX_FRESHNESS_DAYS);
        let note = resolution
            .evidence
            .as_deref()
            .ok_or("a resolved lane carries a note")?;
        for (path, digest) in IL_CAPTURES {
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
    fn coach_lane_il_capture_digests_match_the_held_fixtures() -> TestResult {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for ((path, digest), bytes) in IL_CAPTURES.into_iter().zip(IL_CAPTURE_BYTES) {
            let file = path
                .rsplit('/')
                .next()
                .ok_or("a capture path carries a file name")?;
            let body = std::fs::read(root.join(path))?;
            check!(eq; body.len() as u64, bytes, "{file} byte count");
            check!(eq; format!("{:x}", Sha256::digest(&body)), digest, "{file} sha256");
            check!(COVERAGE_JSON.contains(file), "coverage.json omits {file}");
            check!(
                COVERAGE_JSON.contains(&bytes.to_string()),
                "coverage.json omits {file} byte count {bytes}"
            );
        }
        Ok(())
    }

    #[test]
    fn coach_lane_il_arm_is_wired_into_the_plan_and_the_engine_arm_table() -> TestResult {
        let (plan, armed) = bound()?;
        check!(plan.sweepable.iter().any(|slug| slug == IL_ARM));
        check!(teams_arms::arm_for(IL_ARM).is_some());
        check!(eq; armed.status(), CoachLaneStatus::Armed);
        Ok(())
    }

    #[test]
    fn coach_lane_il_stale_capture_records_a_blocked_refusal_without_closing() -> TestResult {
        check!(eq; freshness_days(IL_CAPTURE_ON, BOUNDARY_ON)?, COACH_LANE_MAX_FRESHNESS_DAYS);
        check!(eq; freshness_days(IL_CAPTURE_ON, STALE_ON)?, COACH_LANE_MAX_FRESHNESS_DAYS + 1);
        let fresh = super::super::evaluate(&POLICY, season()?, 1, BOUNDARY_ON)?;
        check!(eq; fresh.status(), CoachLaneStatus::Armed);
        check!(eq; fresh.resolution().freshness_days, COACH_LANE_MAX_FRESHNESS_DAYS);
        let stale = super::super::evaluate(&POLICY, season()?, 1, STALE_ON)?;
        check!(eq; stale.status(), CoachLaneStatus::Blocked);
        check!(!stale.is_terminal());
        check!(stale.evidence().is_none());
        check!(stale.acceptance().is_none());
        let refusal = stale.refusal().ok_or("a stale capture records a refusal")?;
        check!(refusal.reason().contains("days old"));
        check!(refusal.reason().contains(IL_ABINGDON_PATH));
        let state = JurisdictionState {
            coach_lane: Some(stale),
            ..JurisdictionState::default()
        };
        check!(coach_lane_owed(&state));
        Ok(())
    }

    #[test]
    fn coach_lane_il_remainder_matches_the_reviewed_true_missing() -> TestResult {
        check!(GROUP_A.contains(IL_TRUE_MISSING));
        Ok(())
    }

    #[test]
    fn coach_lane_il_refusal_is_recorded_in_the_durable_plan_path() -> TestResult {
        let (plan, armed) = bound()?;
        check!(plan
            .refused
            .iter()
            .any(|entry| entry.slug == IL_REFUSAL_SLUG && entry.reason == IL_TRUE_MISSING));
        let refusals = plan.coach_lane_refusals(OBSERVED_ON)?;
        check!(refusals
            .iter()
            .any(|refusal| refusal.source().id == IL_REFUSAL_SLUG
                && refusal.reason() == IL_TRUE_MISSING));
        check!(eq; armed.arm_name(), IL_ARM);
        check!(LANES
            .iter()
            .any(|row| row.jurisdiction == UsJurisdiction::Illinois));
        Ok(())
    }
}
