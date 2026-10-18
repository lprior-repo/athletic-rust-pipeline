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
fn coach_lane_sd_policy_hold_is_unresolved_and_still_owed() -> TestResult {
    let (plan, blocked) = bound()?;
    let reason = census_crawl::registry::acquisition_refusal_for_source(SD_ARM)
        .ok_or("bound acquisition hold missing")?;
    check!(eq; blocked.id(), "coach_lane:SD:2026:1");
    check!(eq; blocked.status(), CoachLaneStatus::Blocked);
    check!(!blocked.is_terminal());
    check!(eq; blocked.arm_name(), SD_ARM);
    check!(blocked.evidence().is_none());
    check!(blocked.acceptance().is_none());
    let refusal = blocked.refusal().ok_or("policy hold refusal missing")?;
    check!(eq; refusal.source().id.as_str(), SD_ARM);
    check!(eq; refusal.reason(), reason);
    check!(eq; refusal.observed_on(), OBSERVED_ON);
    check!(eq; blocked.resolution(), &CoachLaneResolution::unresolved(OBSERVED_ON));
    let state = JurisdictionState {
        plan: Some(plan),
        coach_lane: Some(blocked),
        ..JurisdictionState::default()
    };
    check!(coach_lane_owed(&state));
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
        check!(eq; u64::try_from(body.len())?, bytes, "{file} byte count");
        check!(eq; format!("{:x}", Sha256::digest(&body)), digest, "{file} sha256");
        check!(SOURCE_MD.contains(file), "SOURCE.md omits {file}");
        check!(SOURCE_MD.contains(digest), "SOURCE.md omits {digest}");
    }
    Ok(())
}

#[test]
fn coach_lane_sd_historical_arm_stays_wired_but_new_plan_is_not_sweepable() -> TestResult {
    let (plan, blocked) = bound()?;
    check!(!plan.sweepable.iter().any(|slug| slug == SD_ARM));
    check!(teams_arms::arm_for(SD_ARM).is_some());
    check!(eq; blocked.status(), CoachLaneStatus::Blocked);
    Ok(())
}

#[test]
fn coach_lane_sd_policy_hold_precedes_capture_freshness() -> TestResult {
    check!(eq; freshness_days(SD_CAPTURE_ON, BOUNDARY_ON)?, COACH_LANE_MAX_FRESHNESS_DAYS);
    check!(eq; freshness_days(SD_CAPTURE_ON, STALE_ON)?, COACH_LANE_MAX_FRESHNESS_DAYS + 1);
    let reason = census_crawl::registry::acquisition_refusal_for_source(SD_ARM)
        .ok_or("bound acquisition hold missing")?;
    for date in [OBSERVED_ON, BOUNDARY_ON, STALE_ON] {
        let blocked = super::super::evaluate(&POLICY, season()?, 1, date)?;
        check!(eq; blocked.status(), CoachLaneStatus::Blocked);
        check!(!blocked.is_terminal());
        check!(blocked.evidence().is_none());
        check!(blocked.acceptance().is_none());
        let refusal = blocked.refusal().ok_or("policy hold refusal missing")?;
        check!(eq; refusal.reason(), reason);
        check!(eq; refusal.observed_on(), date);
        check!(eq; blocked.resolution(), &CoachLaneResolution::unresolved(date));
    }
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
    let (plan, blocked) = bound()?;
    check!(plan.has_engineering_gaps());
    check!(plan
        .refused
        .iter()
        .any(|entry| entry.slug == SD_REFUSAL_SLUG && entry.reason == SD_TRUE_MISSING));
    let refusals = plan.coach_lane_refusals(OBSERVED_ON)?;
    check!(refusals.iter().any(
        |refusal| refusal.source().id == SD_REFUSAL_SLUG && refusal.reason() == SD_TRUE_MISSING
    ));
    check!(eq; blocked.arm_name(), SD_ARM);
    check!(LANES
        .iter()
        .any(|row| row.jurisdiction == UsJurisdiction::SouthDakota));
    Ok(())
}

fn historical_armed() -> Result<CoachLaneObligation, CoachLaneError> {
    let season = SchoolYear::new(2026).ok_or(CoachLaneError::OutOfRange { field: "season" })?;
    let pending = CoachLaneObligation::pending(PendingCoachLane {
        jurisdiction: UsJurisdiction::SouthDakota,
        season,
        revision: 1,
        arm: SD_ARM.to_string(),
        arm_basis: SD_ARM_BASIS.to_string(),
        observed_on: OBSERVED_ON.to_string(),
    })?;
    let evidence = CoachLaneEvidence::new(
        SD_SCHOOLS,
        SD_CONTACTS,
        SD_YANKTON_GIRLS_DIGEST,
        SD_CAPTURE_ON,
    )?
    .with_source_url(SD_SOURCE_URL)?;
    pending.arm(
        evidence,
        CoachLaneResolution::resolved(
            CoachLaneRole::Published,
            CoachLaneTenure::Current,
            CoachLaneMailbox::Unpublished,
            SD_CAPTURE_ON,
            4,
            CAPTURE.resolved_note(),
        )?,
    )
}

#[test]
fn coach_lane_sd_historical_armed_journal_preserves_capture_history_on_reopen() -> TestResult {
    let directory = tempfile::tempdir()?;
    let historical = historical_armed()?;
    let bytes = serde_json::to_vec(&historical)?;
    {
        let store = census_store::Store::open(directory.path())?;
        store.journal_done("coach-policy-history", historical.id(), &historical)?;
        store.flush()?;
    }
    let store = census_store::Store::open(directory.path())?;
    let value = store
        .journal_payload("coach-policy-history", historical.id())?
        .ok_or("historical armed obligation missing")?;
    let retained: CoachLaneObligation = serde_json::from_value(value)?;
    check!(eq; serde_json::to_vec(&retained)?, bytes);
    check!(eq; retained.status(), CoachLaneStatus::Armed);
    check!(retained.is_terminal());
    let evidence = retained.evidence().ok_or("historical capture missing")?;
    check!(eq; evidence.schools(), SD_SCHOOLS);
    check!(eq; evidence.contacts(), SD_CONTACTS);
    check!(eq; evidence.digest(), SD_YANKTON_GIRLS_DIGEST);
    check!(eq; evidence.observed_on(), SD_CAPTURE_ON);
    check!(eq; evidence.source_url(), Some(SD_SOURCE_URL));
    let note = retained
        .resolution()
        .evidence
        .as_deref()
        .ok_or("historical note missing")?;
    for (path, digest) in SD_CAPTURES {
        check!(note.contains(path));
        check!(note.contains(digest));
    }
    let state = JurisdictionState {
        coach_lane: Some(retained),
        ..JurisdictionState::default()
    };
    check!(!coach_lane_owed(&state));
    check!(eq; super::super::evaluate(&POLICY, season()?, 1, OBSERVED_ON)?.status(), CoachLaneStatus::Blocked);
    check!(eq; serde_json::to_vec(&historical)?, bytes);
    Ok(())
}
