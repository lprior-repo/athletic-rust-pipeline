use super::super::{az, bind, ct, fl, ny};
use crate::restate_services::jurisdiction::coach_lane_owed;
use crate::restate_services::plan::{compute_plan_fingerprint, plan as planned, BrowserLaneState};
use crate::restate_services::wire::{JurisdictionState, SourcePlan};
use census_domain::model::{CoachLaneObligation, CoachLaneStatus, SchoolYear};
use census_domain::UsJurisdiction;
use census_reconcile::identity::Revision;
use census_store::{Store, Table};

type TestResult = Result<(), Box<dyn std::error::Error>>;
const OBSERVED_ON: &str = "2026-10-12";

fn bound(state: UsJurisdiction) -> Result<JurisdictionState, Box<dyn std::error::Error>> {
    let season = SchoolYear::new(2026).ok_or("invalid qualification season")?;
    let browser = BrowserLaneState::Absent;
    let fingerprint = compute_plan_fingerprint(state, season, Revision(1), browser);
    let mut plan = SourcePlan::of(&planned(state, browser), fingerprint);
    let lane = bind(&mut plan, state, season, 1, OBSERVED_ON)?.ok_or("lane missing")?;
    Ok(JurisdictionState {
        identity: format!("jurisdiction:{}:2026-27:1", state.code()),
        plan: Some(plan),
        coach_lane: Some(lane),
        ..JurisdictionState::default()
    })
}

fn assert_owed(state: &JurisdictionState) -> TestResult {
    let lane = state.coach_lane.as_ref().ok_or("persisted lane missing")?;
    check!(eq; lane.status(), CoachLaneStatus::Blocked);
    check!(lane.evidence().is_none());
    check!(lane.acceptance().is_none());
    let refusal = lane.refusal().ok_or("recorded refusal missing")?;
    check!(eq; refusal.observed_on(), OBSERVED_ON);
    let plan = state.plan.as_ref().ok_or("persisted plan missing")?;
    let entries: Vec<_> = plan
        .refused
        .iter()
        .filter(|entry| entry.slug == refusal.source().id)
        .collect();
    check!(eq; entries.len(), 1);
    check!(eq; entries.first().map(|entry| entry.reason.as_str()), Some(refusal.reason()));
    check!(coach_lane_owed(state));
    Ok(())
}

pub(super) fn persisted_refusal(state: UsJurisdiction, slug: &str, reason: &str) -> TestResult {
    let root = tempfile::tempdir()?;
    let state = bound(state)?;
    assert_owed(&state)?;
    let lane = state.coach_lane.as_ref().ok_or("lane missing")?;
    let payload = serde_json::to_value(&state)?;
    let phase = "coach-lane-qualification";
    {
        let store = Store::open(root.path())?;
        store.replace(Table::CoachLanes, lane)?;
        store.journal_done(phase, &state.identity, &payload)?;
        store.flush()?;
    }
    let store = Store::open(root.path())?;
    let retained = store
        .journal_payload(phase, &state.identity)?
        .ok_or("refusal lost")?;
    check!(eq; retained, payload);
    let mut decoded: JurisdictionState = serde_json::from_value(retained)?;
    assert_owed(&decoded)?;
    let row: Vec<CoachLaneObligation> = store.scan(Table::CoachLanes)?;
    check!(eq; row.as_slice(), std::slice::from_ref(lane));
    let refusal = row
        .first()
        .and_then(CoachLaneObligation::refusal)
        .ok_or("refusal missing")?;
    check!(eq; refusal.reason(), reason);
    check!(eq; refusal.source().id.as_str(), slug);
    replay(&store, phase, &mut decoded)?;
    check!(eq; serde_json::to_value(&decoded)?, payload);
    check!(eq; store.walk_table(Table::CoachLanes)?.rows, 1);
    check!(eq; store.journal_keys(phase)?.len(), 1);
    check!(eq; store.scan::<CoachLaneObligation>(Table::CoachLanes)?.as_slice(), row.as_slice());
    check!(eq; store.journal_payload(phase, &decoded.identity)?, Some(payload));
    check!(eq; store.walk_table(Table::Coaches)?.rows, 0);
    Ok(())
}

fn replay(store: &Store, phase: &str, state: &mut JurisdictionState) -> TestResult {
    let prior = state.coach_lane.as_ref().ok_or("lane missing")?;
    let jurisdiction = prior.jurisdiction();
    let season = SchoolYear::new(2026).ok_or("invalid qualification season")?;
    let plan = state.plan.as_mut().ok_or("plan missing")?;
    state.coach_lane = bind(plan, jurisdiction, season, 1, OBSERVED_ON)?;
    let lane = state.coach_lane.as_ref().ok_or("replayed lane missing")?;
    store.replace(Table::CoachLanes, lane)?;
    store.journal_done(phase, &state.identity, state)?;
    assert_owed(state)
}

#[test]
fn coach_lane_ny_qualification_refusal_survives_reopen_and_replay() -> TestResult {
    persisted_refusal(
        UsJurisdiction::NewYork,
        ny::NY_REFUSAL_SLUG,
        ny::NY_TRUE_MISSING,
    )
}

#[test]
fn coach_lane_az_qualification_refusal_survives_reopen_and_replay() -> TestResult {
    persisted_refusal(
        UsJurisdiction::Arizona,
        az::AZ_REFUSAL_SLUG,
        az::AZ_TRUE_MISSING,
    )
}

#[test]
fn coach_lane_ct_phone_only_directory_cannot_close_and_survives_reopen() -> TestResult {
    persisted_refusal(
        UsJurisdiction::Connecticut,
        ct::CT_REFUSAL_SLUG,
        ct::CT_TRUE_MISSING,
    )
}

#[test]
fn coach_lane_fl_aggregate_log_cannot_close_and_survives_reopen() -> TestResult {
    persisted_refusal(
        UsJurisdiction::Florida,
        fl::FL_REFUSAL_SLUG,
        fl::FL_TRUE_MISSING,
    )
}
