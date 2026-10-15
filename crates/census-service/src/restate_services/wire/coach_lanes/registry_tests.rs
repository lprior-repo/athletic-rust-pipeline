use super::bind;
use crate::restate_services::jurisdiction::{coach_lane_owed, owed_work};
use crate::restate_services::plan::{
    compute_plan_fingerprint, plan as planned, BrowserLaneState, RefusalKind,
};
use crate::restate_services::wire::{
    HistoryWindow, JurisdictionRequest, JurisdictionState, SourcePlan,
};
use census_domain::model::Revision;
use census_domain::model::{CoachLaneError, CoachLaneStatus, SchoolYear};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

pub(super) type TestResult = Result<(), Box<dyn std::error::Error>>;
const OBSERVED_ON: &str = "2026-10-12";
const GROUP_A: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../research/sources/coach-coverage-bundle-20261004/state-work-orders/group-a.md"
));
const GROUP_B: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../research/sources/coach-coverage-bundle-20261004/state-work-orders/group-b.md"
));
const GROUP_C: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../research/sources/coach-coverage-bundle-20261004/state-work-orders/group-c.md"
));

fn request(
    jurisdiction: UsJurisdiction,
) -> Result<JurisdictionRequest, Box<dyn std::error::Error>> {
    Ok(JurisdictionRequest {
        jurisdiction,
        season: SchoolYear::new(2026).ok_or("school year 2026")?,
        revision: Revision(1),
        history: HistoryWindow::cohort(OBSERVED_ON)?,
        refresh: false,
        limit_per_state: None,
        concurrency: 1,
        observed_on: Some(OBSERVED_ON.to_string()),
        authorized_hosts: Vec::new(),
        source_parallelism: 1,
    })
}

fn bound(jurisdiction: UsJurisdiction) -> Result<JurisdictionState, Box<dyn std::error::Error>> {
    let request = request(jurisdiction)?;
    let browser = BrowserLaneState::Absent;
    let fingerprint =
        compute_plan_fingerprint(jurisdiction, request.season, request.revision, browser);
    let mut plan = SourcePlan::of(&planned(jurisdiction, browser), fingerprint);
    let lane = bind(&mut plan, jurisdiction, request.season, 1, OBSERVED_ON)?
        .ok_or_else(|| format!("{} has no registered coach decision", jurisdiction.code()))?;
    Ok(JurisdictionState {
        identity: format!("jurisdiction:{}:2026-27:1", jurisdiction.code()),
        plan: Some(plan),
        coach_lane: Some(lane),
        ..JurisdictionState::default()
    })
}

fn expected_status(jurisdiction: UsJurisdiction) -> CoachLaneStatus {
    match jurisdiction {
        UsJurisdiction::Utah => CoachLaneStatus::Armed,
        _ => CoachLaneStatus::Blocked,
    }
}

fn expected_kind(jurisdiction: UsJurisdiction) -> RefusalKind {
    match jurisdiction {
        UsJurisdiction::Alabama
        | UsJurisdiction::Iowa
        | UsJurisdiction::Mississippi
        | UsJurisdiction::Missouri => RefusalKind::AccessBlocked,
        _ => RefusalKind::EngineeringGap,
    }
}

fn work_order_reason(
    section: &str,
    jurisdiction: UsJurisdiction,
) -> Result<String, Box<dyn std::error::Error>> {
    if let Some((_, reason)) = section.split_once("4. **True missing**: ") {
        return Ok(reason
            .lines()
            .next()
            .ok_or("true-missing line absent")?
            .to_string());
    }
    let (_, after) = section
        .split_once("### 4. True missing")
        .ok_or("missing heading")?;
    let (_, after) = after.split_once('\n').ok_or("missing reason line")?;
    let reason = after
        .split_once("### 5.")
        .ok_or("missing notes heading")?
        .0
        .trim();
    match jurisdiction {
        UsJurisdiction::Nevada
        | UsJurisdiction::Oregon
        | UsJurisdiction::SouthDakota
        | UsJurisdiction::Utah
        | UsJurisdiction::Vermont => Ok(reason
            .lines()
            .map(|line| line.trim_start_matches("- ").trim_end_matches('.'))
            .collect::<Vec<_>>()
            .join(" · ")),
        _ => Ok(reason.to_string()),
    }
}

fn assert_decision(state: &JurisdictionState, expected_reason: &str) -> TestResult {
    let lane = state.coach_lane.as_ref().ok_or("coach decision missing")?;
    let jurisdiction = lane.jurisdiction();
    let slug = format!(
        "{}_missing_coach_sources",
        jurisdiction.code().to_lowercase()
    );
    let plan = state.plan.as_ref().ok_or("source plan missing")?;
    let refusals: Vec<_> = plan
        .refused
        .iter()
        .filter(|entry| entry.slug == slug)
        .collect();
    check!(eq; refusals.len(), 1);
    let refusal = refusals.first().ok_or("registered refusal missing")?;
    check!(eq; refusal.reason.as_str(), expected_reason);
    check!(eq; refusal.kind, expected_kind(jurisdiction));
    check!(eq; lane.id(), format!("coach_lane:{}:2026:1", jurisdiction.code()));
    check!(eq; lane.status(), expected_status(jurisdiction));
    let (owed, reasons) = owed_work(state, &request(jurisdiction)?);
    check!(reasons.contains(&format!("refused {slug}: {expected_reason}")));
    match expected_status(jurisdiction) {
        CoachLaneStatus::Armed => {
            let evidence = lane.evidence().ok_or("armed obligation has no proof")?;
            check!(evidence.contacts() > 0);
            check!(evidence.schools() > 0);
            check!(!coach_lane_owed(state));
            check!(!owed.iter().any(|stage| stage == "coach_lane"));
            check!(plan.sweepable.iter().any(|arm| arm == lane.arm_name()));
            check!(crate::restate_services::teams_arms::arm_for(lane.arm_name()).is_some());
        }
        CoachLaneStatus::Blocked => {
            let refusal = lane.refusal().ok_or("blocked obligation has no refusal")?;
            let (lane_source, lane_reason) = if jurisdiction == UsJurisdiction::SouthDakota {
                (
                    "bound",
                    census_crawl::registry::acquisition_refusal_for_source("bound")
                        .ok_or("bound acquisition hold missing")?,
                )
            } else {
                (slug.as_str(), expected_reason)
            };
            check!(eq; refusal.source().id.as_str(), lane_source);
            check!(eq; refusal.reason(), lane_reason);
            check!(eq; refusal.observed_on(), OBSERVED_ON);
            check!(lane.evidence().is_none());
            check!(lane.acceptance().is_none());
            check!(coach_lane_owed(state));
            check!(owed.iter().any(|stage| stage == "coach_lane"));
            check!(reasons.contains(&format!(
                "coach_lane: blocked under arm {} (updated {OBSERVED_ON})",
                lane.arm_name()
            )));
        }
        status => return Err(format!("unexpected expected status {status}").into()),
    }
    Ok(())
}

fn check_group(work_order: &str, expected_count: usize) -> TestResult {
    let root = tempfile::tempdir()?;
    let mut retained = Vec::new();
    {
        let store = Store::open(root.path())?;
        for section in work_order.split("\n## ").skip(1) {
            let code = section
                .split_whitespace()
                .next()
                .ok_or("missing jurisdiction code")?;
            let Some(jurisdiction) = UsJurisdiction::CENSUS_SCOPE
                .iter()
                .copied()
                .find(|jurisdiction| jurisdiction.code() == code)
            else {
                continue;
            };
            let expected = work_order_reason(section, jurisdiction)?;
            let mut state = bound(jurisdiction)?;
            assert_decision(&state, &expected)?;
            let before = serde_json::to_vec(&state)?;
            for _ in 0..3 {
                state.coach_lane = bind(
                    state.plan.as_mut().ok_or("source plan missing")?,
                    jurisdiction,
                    request(jurisdiction)?.season,
                    1,
                    OBSERVED_ON,
                )?;
                check!(eq; serde_json::to_vec(&state)?, before);
                assert_decision(&state, &expected)?;
            }
            store.journal_done("coach-registry-qualification", &state.identity, &state)?;
            retained.push((state.identity, before, expected));
        }
        check!(eq; retained.len(), expected_count);
        store.flush()?;
    }
    let store = Store::open(root.path())?;
    check!(eq; store.journal_keys("coach-registry-qualification")?.len(), expected_count);
    for (identity, before, expected) in retained {
        let payload = store
            .journal_payload("coach-registry-qualification", &identity)?
            .ok_or("coach decision lost on reopen")?;
        let state: JurisdictionState = serde_json::from_value(payload)?;
        check!(eq; serde_json::to_vec(&state)?, before);
        assert_decision(&state, &expected)?;
        store.journal_done("coach-registry-qualification", &identity, &state)?;
    }
    check!(eq; store.journal_keys("coach-registry-qualification")?.len(), expected_count);
    check!(store
        .stats()?
        .tables
        .contains(&(Table::Coaches.file().to_string(), 0)));
    Ok(())
}

#[test]
fn coach_registry_group_a_rebind_and_reopen_preserve_exact_decisions() -> TestResult {
    check_group(GROUP_A, 15)
}

#[test]
fn coach_registry_group_b_rebind_and_reopen_preserve_exact_decisions() -> TestResult {
    check_group(GROUP_B, 17)
}

#[test]
fn coach_registry_group_c_rebind_and_reopen_preserve_exact_decisions() -> TestResult {
    check_group(GROUP_C, 17)
}

#[test]
fn every_in_scope_jurisdiction_coach_lane_is_armed_or_refused_by_name() -> TestResult {
    check!(eq; UsJurisdiction::CENSUS_SCOPE.len(), 49);
    for jurisdiction in UsJurisdiction::CENSUS_SCOPE {
        let state = bound(jurisdiction)?;
        let lane = state.coach_lane.ok_or("coach decision missing")?;
        check!(eq; lane.jurisdiction(), jurisdiction);
        check!(eq; lane.status(), expected_status(jurisdiction));
    }
    Ok(())
}

#[test]
fn coach_registry_failed_binding_leaves_serialized_plan_unchanged() -> TestResult {
    let jurisdiction = UsJurisdiction::Utah;
    let season = request(jurisdiction)?.season;
    let mut plan = SourcePlan::default();
    let before = serde_json::to_vec(&plan)?;
    check!(eq; bind(&mut plan, jurisdiction, season, 1, OBSERVED_ON),
        Err(CoachLaneError::OutOfRange { field: "arm" }));
    check!(eq; serde_json::to_vec(&plan)?, before);
    Ok(())
}
