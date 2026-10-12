use census_domain::model::{
    CoachLaneError, CoachLaneEvidence, CoachLaneMailbox, CoachLaneObligation, CoachLaneRefusal,
    CoachLaneResolution, CoachLaneRole, CoachLaneStage, CoachLaneTenure, PendingCoachLane,
    SchoolYear, SourceRef, COACH_LANE_MAX_FRESHNESS_DAYS,
};
use census_domain::UsJurisdiction;

use super::freshness_days;

pub(super) const SD_ARM: &str = "bound";
pub(super) const SD_REFUSAL_SLUG: &str = "sd_missing_coach_sources";
pub(super) const SD_TRUE_MISSING: &str = "`www.gobound.com/sd/sdhsaa/...` staff pages for schools not among the 560 held rows (gobound live was refused/listed as blocked) · SDHSAA member directory itself (SRC-224) — not held; school-side directory (SD DOE naming) · School district sites for coach emails (SDHSAA publishes none)";
pub(super) const SD_YANKTON_GIRLS_PATH: &str =
    "crates/census-crawl/tests/fixtures/bound/staff-yankton-sd-girls-xc.html";
pub(super) const SD_YANKTON_GIRLS_DIGEST: &str =
    "3bdab3bd25bbc3ae5ff06c308922b7bd8480a303877abbfcd3aecf32cd5dc93f";
pub(super) const SD_YANKTON_BOYS_PATH: &str =
    "crates/census-crawl/tests/fixtures/bound/staff-yankton-sd-boys-xc.html";
pub(super) const SD_YANKTON_BOYS_DIGEST: &str =
    "55f0a5027ed504c15a2003c8a0eb3de48cee35cff9f615c05ca9972f9d076ccc";
pub(super) const SD_WHITERIVER_PATH: &str =
    "crates/census-crawl/tests/fixtures/bound/staff-whiteriver-sd-girls-xc.html";
pub(super) const SD_WHITERIVER_DIGEST: &str =
    "d8d0353e15399fae06426ff15d7ed27834b59be0f255003bc1d3eb42967064f5";
pub(super) const SD_FAITH_PATH: &str =
    "crates/census-crawl/tests/fixtures/bound/staff-faith-sd-boys-xc-empty-staff.html";
pub(super) const SD_FAITH_DIGEST: &str =
    "c4a3d7b76a7d4bebd45ca04e039d6071b7f26f35191fed845ee57d37c76efcf2";
pub(super) const SD_CAPTURES: [(&str, &str); 4] = [
    (SD_YANKTON_GIRLS_PATH, SD_YANKTON_GIRLS_DIGEST),
    (SD_YANKTON_BOYS_PATH, SD_YANKTON_BOYS_DIGEST),
    (SD_WHITERIVER_PATH, SD_WHITERIVER_DIGEST),
    (SD_FAITH_PATH, SD_FAITH_DIGEST),
];
pub(super) const SD_SOURCE_URL: &str =
    "https://www.gobound.com/sd/sdhsaa/girlscrosscountry/2026-27/yankton/v/staff";
pub(super) const SD_SCHOOLS: u32 = 3;
pub(super) const SD_CONTACTS: u32 = 6;
pub(super) const SD_CAPTURE_ON: &str = "2026-10-07";

const SD_ARM_BASIS: &str = "the bound descriptor declares school-coach contact and is admitted for South Dakota, and TeamsArm::Bound walks each SDHSAA school's GoBound staff pages; the retained SD staff captures are the held output of that walk";
const SD_RESOLUTION_NOTE: &str = "four retained 2026-27-season GoBound SDHSAA staff captures (Yankton boys and girls cross country, White River girls cross country, Faith boys cross country - the retained empty-staff edge) list Head Coach and Assistant Coach rows and publish no mailbox; the crawl cache records no fetch timestamp, so 2026-10-07 is their documented offline replay date (research/sources/bound/SOURCE_REPORT.md):";

fn resolved_note() -> String {
    let mut note = String::from(SD_RESOLUTION_NOTE);
    for (path, digest) in SD_CAPTURES {
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
        jurisdiction: UsJurisdiction::SouthDakota,
        season,
        revision,
        arm: SD_ARM.to_string(),
        arm_basis: SD_ARM_BASIS.to_string(),
        observed_on: observed_on.to_string(),
    })?;
    let freshness = freshness_days(SD_CAPTURE_ON, observed_on)?;
    if freshness > COACH_LANE_MAX_FRESHNESS_DAYS {
        return pending.record(
            CoachLaneStage::Blocked,
            Some(stale_refusal(freshness, observed_on)?),
            CoachLaneResolution::unresolved(observed_on),
        );
    }
    let evidence = CoachLaneEvidence::new(
        SD_SCHOOLS,
        SD_CONTACTS,
        SD_YANKTON_GIRLS_DIGEST,
        SD_CAPTURE_ON,
    )?
    .with_source_url(SD_SOURCE_URL)?;
    let resolution = CoachLaneResolution::resolved(
        CoachLaneRole::Published,
        CoachLaneTenure::Current,
        CoachLaneMailbox::Unpublished,
        SD_CAPTURE_ON,
        freshness,
        resolved_note(),
    )?;
    pending.arm(evidence, resolution)
}

fn stale_refusal(freshness: u32, observed_on: &str) -> Result<CoachLaneRefusal, CoachLaneError> {
    CoachLaneRefusal::new(
        SourceRef::id(SD_ARM),
        format!(
            "the held bound capture {SD_YANKTON_GIRLS_PATH} is {freshness} days old at lane creation, beyond the {COACH_LANE_MAX_FRESHNESS_DAYS}-day bound, so the South Dakota arm cannot retain it"
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
    const BOUNDARY_ON: &str = "2027-11-11";
    const STALE_ON: &str = "2027-11-12";
    const SD_CAPTURE_BYTES: [u64; 4] = [49_944, 49_884, 49_705, 48_167];
    const WORK_ORDER: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/sources/coach-coverage-bundle-20261004/state-work-orders/group-c.md"
    ));
    const SOURCE_MD: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/census-crawl/tests/fixtures/bound/SOURCE.md"
    ));

    fn season() -> Result<SchoolYear, Box<dyn std::error::Error>> {
        Ok(SchoolYear::new(2026).ok_or("school year 2026")?)
    }

    fn bound() -> Result<(SourcePlan, CoachLaneObligation), Box<dyn std::error::Error>> {
        let mut plan = SourcePlan::of(
            &planned(UsJurisdiction::SouthDakota, BrowserLaneState::Absent),
            "fixture".to_string(),
        );
        let obligation = bind(
            &mut plan,
            UsJurisdiction::SouthDakota,
            season()?,
            1,
            OBSERVED_ON,
        )?
        .ok_or("the South Dakota lane is registered")?;
        Ok((plan, obligation))
    }

    #[test]
    fn coach_lane_sd_arms_on_the_retained_bound_captures() -> TestResult {
        let (plan, armed) = bound()?;
        check!(eq; armed.id(), "coach_lane:SD:2026:1");
        check!(eq; armed.status(), CoachLaneStatus::Armed);
        check!(armed.is_terminal());
        check!(eq; armed.arm_name(), SD_ARM);
        check!(armed.refusal().is_none());
        check!(armed.acceptance().is_none());
        let evidence = armed.evidence().ok_or("an armed lane carries evidence")?;
        check!(eq; evidence.schools(), SD_SCHOOLS);
        check!(eq; evidence.contacts(), SD_CONTACTS);
        check!(eq; evidence.digest(), SD_YANKTON_GIRLS_DIGEST);
        check!(eq; evidence.observed_on(), SD_CAPTURE_ON);
        check!(eq; evidence.source_url(), Some(SD_SOURCE_URL));
        let resolution = armed.resolution();
        check!(eq; resolution.role, CoachLaneRole::Published);
        check!(eq; resolution.tenure, CoachLaneTenure::Current);
        check!(eq; resolution.mailbox, CoachLaneMailbox::Unpublished);
        check!(eq; resolution.retrieved_on, SD_CAPTURE_ON);
        check!(eq; resolution.freshness_days, 4);
        check!(resolution.freshness_days <= COACH_LANE_MAX_FRESHNESS_DAYS);
        let note = resolution
            .evidence
            .as_deref()
            .ok_or("a resolved lane carries a note")?;
        for (path, digest) in SD_CAPTURES {
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
    fn coach_lane_sd_capture_digests_match_the_held_fixtures() -> TestResult {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for ((path, digest), bytes) in SD_CAPTURES.into_iter().zip(SD_CAPTURE_BYTES) {
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
    fn coach_lane_sd_arm_is_wired_into_the_plan_and_the_engine_arm_table() -> TestResult {
        let (plan, armed) = bound()?;
        check!(plan.sweepable.iter().any(|slug| slug == SD_ARM));
        check!(teams_arms::arm_for(SD_ARM).is_some());
        check!(eq; armed.status(), CoachLaneStatus::Armed);
        Ok(())
    }

    #[test]
    fn coach_lane_sd_stale_capture_records_a_blocked_refusal_without_closing() -> TestResult {
        check!(eq; freshness_days(SD_CAPTURE_ON, BOUNDARY_ON)?, COACH_LANE_MAX_FRESHNESS_DAYS);
        check!(eq; freshness_days(SD_CAPTURE_ON, STALE_ON)?, COACH_LANE_MAX_FRESHNESS_DAYS + 1);
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
        check!(refusal.reason().contains(SD_YANKTON_GIRLS_PATH));
        let state = JurisdictionState {
            coach_lane: Some(stale),
            ..JurisdictionState::default()
        };
        check!(coach_lane_owed(&state));
        Ok(())
    }

    #[test]
    fn coach_lane_sd_remainder_is_the_work_order_true_missing() -> TestResult {
        for fragment in SD_TRUE_MISSING.split(" · ") {
            check!(
                WORK_ORDER.contains(fragment),
                "the group-c work order does not carry this bullet: {fragment}"
            );
        }
        Ok(())
    }

    #[test]
    fn coach_lane_sd_refusal_is_recorded_in_the_durable_plan_path() -> TestResult {
        let (plan, armed) = bound()?;
        check!(plan.has_engineering_gaps());
        check!(plan
            .refused
            .iter()
            .any(|entry| entry.slug == SD_REFUSAL_SLUG && entry.reason == SD_TRUE_MISSING));
        let refusals = plan.coach_lane_refusals(OBSERVED_ON)?;
        check!(refusals
            .iter()
            .any(|refusal| refusal.source().id == SD_REFUSAL_SLUG
                && refusal.reason() == SD_TRUE_MISSING));
        check!(eq; armed.arm_name(), SD_ARM);
        check!(LANES
            .iter()
            .any(|row| row.jurisdiction == UsJurisdiction::SouthDakota));
        Ok(())
    }
}
