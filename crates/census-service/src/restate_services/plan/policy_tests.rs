use super::*;
use crate::restate_services::jurisdiction::{coach_lane_owed, owed_work};
use crate::restate_services::wire::{
    coach_lanes, HistoryWindow, JurisdictionRequest, JurisdictionState, SourcePlan,
};
use census_domain::model::CoachLaneStatus;
use census_store::Store;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const HELD_REASON: &str = "GoBound acquisition is policy-held: retained qualification relied on a browser-class User-Agent, and the 2026-10-12 non-browser content requests were refused. No compliant live route is qualified; retained artifacts may be replayed offline.";
const OBSERVED_ON: &str = "2026-10-14";

#[test]
fn gobound_new_plans_refuse_exact_policy_reason_before_the_armed_strategy() -> TestResult {
    check!(eq; census_crawl::registry::acquisition_refusal_for_source("bound"), Some(HELD_REASON));
    check!(eq; strategies::strategy("bound"), strategies::SourceStrategy::Teams);
    check!(eq; Dispatch::of("bound"), Dispatch::Wired);
    for jurisdiction in [UsJurisdiction::Iowa, UsJurisdiction::SouthDakota] {
        for lane in [BrowserLaneState::Absent, BrowserLaneState::Configured] {
            let dispositions = plan(jurisdiction, lane);
            let bound: Vec<_> = dispositions
                .iter()
                .filter(|unit| unit.slug() == "bound")
                .collect();
            check!(eq; bound.len(), 1);
            let UnitDisposition::Refused(refusal) = bound.first().ok_or("bound missing")? else {
                return Err("bound was sweepable under an acquisition hold".into());
            };
            check!(eq; refusal.kind, RefusalKind::AccessBlocked);
            check!(eq; refusal.reason, HELD_REASON);
            check!(!sweepable(&dispositions)
                .iter()
                .any(|unit| unit.slug == "bound"));
            let recorded = SourcePlan::of(&dispositions, String::new());
            let retained = recorded
                .refused
                .iter()
                .find(|entry| entry.slug == "bound")
                .ok_or("bound refusal not retained")?;
            check!(eq; retained.kind, RefusalKind::AccessBlocked);
            check!(eq; retained.reason.as_str(), HELD_REASON);
        }
    }
    Ok(())
}

#[test]
fn gobound_plan_fingerprints_keep_the_historical_identity_bytes() -> TestResult {
    let season = SchoolYear::new(2026).ok_or("fixture season invalid")?;
    for (jurisdiction, lane, expected) in [
        (
            UsJurisdiction::Iowa,
            BrowserLaneState::Absent,
            "0bc2975258a475357fb5951646f81d0b5d1ad80098a43b24d386292d9dabddaf",
        ),
        (
            UsJurisdiction::Iowa,
            BrowserLaneState::Configured,
            "37aac97977cc81182431e9372830e81cbb683a8e33df28c17d1eff5d20ab51e8",
        ),
        (
            UsJurisdiction::SouthDakota,
            BrowserLaneState::Absent,
            "74e72d347754a37d7434f570d4f8985eae4fafb79a199bf61d32d9aa3c8189ac",
        ),
        (
            UsJurisdiction::SouthDakota,
            BrowserLaneState::Configured,
            "a769566caf3f9c5223261b6f42b52b96f909fe5d7cfa95ff3ffdfd5b0d945584",
        ),
    ] {
        check!(eq; compute_plan_fingerprint(jurisdiction, season, Revision(1), lane), expected);
    }
    Ok(())
}

#[test]
fn gobound_historical_sweepable_plan_journal_is_not_reclassified_on_reopen() -> TestResult {
    let directory = tempfile::tempdir()?;
    let season = SchoolYear::new(2026).ok_or("fixture season invalid")?;
    let mut retained = Vec::new();
    {
        let store = Store::open(directory.path())?;
        for jurisdiction in [UsJurisdiction::Iowa, UsJurisdiction::SouthDakota] {
            let fingerprint = compute_plan_fingerprint(
                jurisdiction,
                season,
                Revision(1),
                BrowserLaneState::Absent,
            );
            let historical = SourcePlan {
                sweepable: vec!["bound".to_string()],
                refused: Vec::new(),
                fingerprint,
            };
            store.journal_done("gobound-historical-plan", jurisdiction.code(), &historical)?;
            retained.push((jurisdiction, serde_json::to_vec(&historical)?));
        }
        store.flush()?;
    }
    let store = Store::open(directory.path())?;
    for (jurisdiction, before) in retained {
        let payload = store
            .journal_payload("gobound-historical-plan", jurisdiction.code())?
            .ok_or("historical plan missing")?;
        let historical: SourcePlan = serde_json::from_value(payload)?;
        check!(eq; serde_json::to_vec(&historical)?, before);
        check!(eq; historical.sweepable, vec!["bound".to_string()]);
        check!(historical.refused.is_empty());
        let fresh = SourcePlan::of(
            &plan(jurisdiction, BrowserLaneState::Absent),
            historical.fingerprint.clone(),
        );
        check!(eq; fresh.fingerprint, historical.fingerprint);
        check!(!fresh.sweepable.iter().any(|source| source == "bound"));
        check!(fresh.refused.iter().any(|source| source.slug == "bound"
            && source.kind == RefusalKind::AccessBlocked
            && source.reason == HELD_REASON));
    }
    Ok(())
}

#[test]
fn gobound_blocked_iowa_and_south_dakota_lanes_remain_owed_with_source_refusal() -> TestResult {
    for jurisdiction in [UsJurisdiction::Iowa, UsJurisdiction::SouthDakota] {
        let request = JurisdictionRequest {
            jurisdiction,
            season: SchoolYear::new(2026).ok_or("fixture season invalid")?,
            revision: Revision(1),
            history: HistoryWindow::cohort(OBSERVED_ON)?,
            refresh: false,
            limit_per_state: None,
            concurrency: 1,
            observed_on: Some(OBSERVED_ON.to_string()),
            authorized_hosts: Vec::new(),
            source_parallelism: 1,
        };
        let mut recorded = SourcePlan::of(
            &plan(jurisdiction, BrowserLaneState::Absent),
            compute_plan_fingerprint(
                jurisdiction,
                request.season,
                request.revision,
                BrowserLaneState::Absent,
            ),
        );
        let lane = coach_lanes::bind(&mut recorded, jurisdiction, request.season, 1, OBSERVED_ON)?
            .ok_or("coach obligation missing")?;
        check!(eq; lane.status(), CoachLaneStatus::Blocked);
        check!(!lane.is_terminal());
        check!(lane.evidence().is_none());
        check!(lane.acceptance().is_none());
        let state = JurisdictionState {
            plan: Some(recorded),
            coach_lane: Some(lane),
            ..JurisdictionState::default()
        };
        check!(coach_lane_owed(&state));
        let (owed, reasons) = owed_work(&state, &request);
        check!(owed.iter().any(|stage| stage == "coach_lane"));
        check!(reasons.contains(&format!("refused bound: {HELD_REASON}")));
    }
    Ok(())
}
