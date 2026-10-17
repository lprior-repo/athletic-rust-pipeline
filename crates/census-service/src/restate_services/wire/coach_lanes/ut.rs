#[cfg(test)]
use census_domain::model::{CoachLaneObligation, CoachLaneRole, CoachLaneTenure, SchoolYear, COACH_LANE_MAX_FRESHNESS_DAYS};
use census_domain::model::CoachLaneMailbox;
use census_domain::UsJurisdiction;

use super::policy::{ArmBasis, Lane, LanePolicy, RetainedCapture};
use crate::restate_services::plan::RefusalKind;

pub(super) const UT_ARM: &str = "uhsaa";
pub(super) const UT_REFUSAL_SLUG: &str = "ut_missing_coach_sources";
pub(super) const UT_TRUE_MISSING: &str = "UHSAA per-school profiles for all 162 (only American Fork + partial in today's taps) · USBE Utah Schools Directory / CACTUS endpoint (`schools.utah.gov`, SRC-165/167/168 — DERIVED only) for addresses · School-site staff directories for emails (Alpine, Davis, Nebo, Weber districts) — lane exists at 9.3 KB";
pub(super) const UT_DIRECTORY_PATH: &str =
    "crates/census-crawl/tests/fixtures/uhsaa/directory.html";
pub(super) const UT_DIRECTORY_DIGEST: &str =
    "3ed900a99bd11aa577bf15fe975a0920748780756c94ec19f04c28a214152d41";
pub(super) const UT_ALTA_PATH: &str = "crates/census-crawl/tests/fixtures/uhsaa/profile-alta.html";
pub(super) const UT_ALTA_DIGEST: &str =
    "6cf4845fab4f61e2a13322411764547d0e344b63e7ee55b71c97008a8e7418bf";
pub(super) const UT_MURRAY_PATH: &str =
    "crates/census-crawl/tests/fixtures/uhsaa/profile-murray.html";
pub(super) const UT_MURRAY_DIGEST: &str =
    "fa37204cfce4e2db4db79d8c7742e52db1903f06bad2c519e435d55025f45d42";
pub(super) const UT_HERRIMAN_PATH: &str =
    "crates/census-crawl/tests/fixtures/uhsaa/profile-herriman.html";
pub(super) const UT_HERRIMAN_DIGEST: &str =
    "aba002e9700ac87449f8f65cbabc192769cb1fccc5912aa3200b2371628b6367";
pub(super) const UT_CAPTURES: [(&str, &str); 4] = [
    (UT_DIRECTORY_PATH, UT_DIRECTORY_DIGEST),
    (UT_ALTA_PATH, UT_ALTA_DIGEST),
    (UT_MURRAY_PATH, UT_MURRAY_DIGEST),
    (UT_HERRIMAN_PATH, UT_HERRIMAN_DIGEST),
];
pub(super) const UT_SOURCE_URL: &str = "https://uhsaa.org/school-directory-new/";
pub(super) const UT_SCHOOLS: u32 = 162;
pub(super) const UT_CONTACTS: u32 = 12;
pub(super) const UT_CAPTURE_ON: &str = "2026-10-04";

const UT_ARM_BASIS: &str = "the uhsaa descriptor declares school-coach contact, is admitted for Utah only, and TeamsArm::UhsaaDirectory walks the member directory and each school profile; the retained UHSAA captures are the held output of that walk";
const UT_RESOLUTION_NOTE: &str = "the retained UHSAA 2026-10-04 captures: the member directory holds 324 anchors over 162 distinct schools, and the Alta, Murray and Herriman profiles publish a head coach with a mailto address for boys and girls cross country and track & field (twelve TF/XC rows over five coaches); profiles for the other 159 members are not part of this evidence set:";

const CAPTURE: RetainedCapture = RetainedCapture {
    schools: UT_SCHOOLS,
    contacts: UT_CONTACTS,
    digest: UT_DIRECTORY_DIGEST,
    captured_on: UT_CAPTURE_ON,
    source_url: UT_SOURCE_URL,
    mailbox: CoachLaneMailbox::Published,
    note: UT_RESOLUTION_NOTE,
    captures: &UT_CAPTURES,
    stale_path: UT_DIRECTORY_PATH,
    jurisdiction_name: "Utah",
};

pub(super) const POLICY: Lane = Lane {
    jurisdiction: UsJurisdiction::Utah,
    arm: UT_ARM,
    refusal: (UT_REFUSAL_SLUG, UT_TRUE_MISSING),
    kind: RefusalKind::EngineeringGap,
    basis: ArmBasis::Text(UT_ARM_BASIS),
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
    const BOUNDARY_ON: &str = "2027-11-08";
    const STALE_ON: &str = "2027-11-09";
    const UT_CAPTURE_BYTES: [u64; 4] = [484_909, 458_367, 456_574, 456_554];
    const WORK_ORDER: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/sources/coach-coverage-bundle-20261004/state-work-orders/group-c.md"
    ));
    const SOURCE_MD: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/census-crawl/tests/fixtures/uhsaa/SOURCE.md"
    ));

    fn season() -> Result<SchoolYear, Box<dyn std::error::Error>> {
        Ok(SchoolYear::new(2026).ok_or("school year 2026")?)
    }

    fn bound() -> Result<(SourcePlan, CoachLaneObligation), Box<dyn std::error::Error>> {
        let mut plan = SourcePlan::of(
            &planned(UsJurisdiction::Utah, BrowserLaneState::Absent),
            "fixture".to_string(),
        );
        let obligation = bind(&mut plan, UsJurisdiction::Utah, season()?, 1, OBSERVED_ON)?
            .ok_or("the Utah lane is registered")?;
        Ok((plan, obligation))
    }

    #[test]
    fn coach_lane_ut_arms_on_the_retained_uhsaa_captures() -> TestResult {
        let (plan, armed) = bound()?;
        check!(eq; armed.id(), "coach_lane:UT:2026:1");
        check!(eq; armed.status(), CoachLaneStatus::Armed);
        check!(armed.is_terminal());
        check!(eq; armed.arm_name(), UT_ARM);
        check!(armed.refusal().is_none());
        check!(armed.acceptance().is_none());
        let evidence = armed.evidence().ok_or("an armed lane carries evidence")?;
        check!(eq; evidence.schools(), UT_SCHOOLS);
        check!(eq; evidence.contacts(), UT_CONTACTS);
        check!(eq; evidence.digest(), UT_DIRECTORY_DIGEST);
        check!(eq; evidence.observed_on(), UT_CAPTURE_ON);
        check!(eq; evidence.source_url(), Some(UT_SOURCE_URL));
        let resolution = armed.resolution();
        check!(eq; resolution.role, CoachLaneRole::Published);
        check!(eq; resolution.tenure, CoachLaneTenure::Current);
        check!(eq; resolution.mailbox, CoachLaneMailbox::Published);
        check!(eq; resolution.retrieved_on, UT_CAPTURE_ON);
        check!(eq; resolution.freshness_days, 7);
        check!(resolution.freshness_days <= COACH_LANE_MAX_FRESHNESS_DAYS);
        let note = resolution
            .evidence
            .as_deref()
            .ok_or("a resolved lane carries a note")?;
        for (path, digest) in UT_CAPTURES {
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
    fn coach_lane_ut_capture_digests_match_the_held_fixtures() -> TestResult {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for ((path, digest), bytes) in UT_CAPTURES.into_iter().zip(UT_CAPTURE_BYTES) {
            let file = path
                .rsplit('/')
                .next()
                .ok_or("a capture path carries a file name")?;
            let body = std::fs::read(root.join(path))?;
            check!(eq; body.len() as u64, bytes, "{file} byte count");
            check!(eq; format!("{:x}", Sha256::digest(&body)), digest, "{file} sha256");
            check!(SOURCE_MD.contains(file), "SOURCE.md omits {file}");
            check!(SOURCE_MD.contains(digest), "SOURCE.md omits {digest}");
        }
        Ok(())
    }

    #[test]
    fn coach_lane_ut_arm_is_wired_into_the_plan_and_the_engine_arm_table() -> TestResult {
        let (plan, armed) = bound()?;
        check!(plan.sweepable.iter().any(|slug| slug == UT_ARM));
        check!(teams_arms::arm_for(UT_ARM).is_some());
        check!(eq; armed.status(), CoachLaneStatus::Armed);
        Ok(())
    }

    #[test]
    fn coach_lane_ut_stale_capture_records_a_blocked_refusal_without_closing() -> TestResult {
        check!(eq; freshness_days(UT_CAPTURE_ON, BOUNDARY_ON)?, COACH_LANE_MAX_FRESHNESS_DAYS);
        check!(eq; freshness_days(UT_CAPTURE_ON, STALE_ON)?, COACH_LANE_MAX_FRESHNESS_DAYS + 1);
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
        check!(refusal.reason().contains(UT_DIRECTORY_PATH));
        let state = JurisdictionState {
            coach_lane: Some(stale),
            ..JurisdictionState::default()
        };
        check!(coach_lane_owed(&state));
        Ok(())
    }

    #[test]
    fn coach_lane_ut_remainder_is_the_work_order_true_missing() -> TestResult {
        for fragment in UT_TRUE_MISSING.split(" · ") {
            check!(
                WORK_ORDER.contains(fragment),
                "the group-c work order does not carry this bullet: {fragment}"
            );
        }
        Ok(())
    }

    #[test]
    fn coach_lane_ut_refusal_is_recorded_in_the_durable_plan_path() -> TestResult {
        let (plan, armed) = bound()?;
        check!(plan
            .refused
            .iter()
            .any(|entry| entry.slug == UT_REFUSAL_SLUG && entry.reason == UT_TRUE_MISSING));
        let refusals = plan.coach_lane_refusals(OBSERVED_ON)?;
        check!(refusals
            .iter()
            .any(|refusal| refusal.source().id == UT_REFUSAL_SLUG
                && refusal.reason() == UT_TRUE_MISSING));
        check!(eq; armed.arm_name(), UT_ARM);
        check!(LANES
            .iter()
            .any(|row| row.jurisdiction == UsJurisdiction::Utah));
        Ok(())
    }
}
