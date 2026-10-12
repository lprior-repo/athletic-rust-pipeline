use crate::restate_services::jurisdiction::{coach_lane_owed, owed_work};
use crate::restate_services::plan::RefusalKind;
use crate::restate_services::wire::{
    ContactStatus, HistoryWindow, JurisdictionRequest, JurisdictionState, RefusedSource,
    SourcePlan, StageOutcome, TeamsSourceOutcome,
};
use census_domain::model::{
    CoachLaneAcceptance, CoachLaneEvidence, CoachLaneObligation, CoachLaneRefusal,
    CoachLaneResolution, CoachLaneStage, CoachLaneStatus, PendingCoachLane, SchoolYear, SourceRef,
};
use census_domain::UsJurisdiction;
use census_reconcile::identity::Revision;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const OBSERVED_ON: &str = "2026-10-11";
const DIGEST: &str = "9f2b1c4d5e6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c";

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
        observed_on: None,
        authorized_hosts: Vec::new(),
        source_parallelism: 1,
    })
}

fn pending(
    jurisdiction: UsJurisdiction,
    arm: &str,
) -> Result<CoachLaneObligation, Box<dyn std::error::Error>> {
    Ok(CoachLaneObligation::pending(PendingCoachLane {
        jurisdiction,
        season: SchoolYear::new(2026).ok_or("school year 2026")?,
        revision: 1,
        arm: arm.to_string(),
        arm_basis: "association directory lists coach roles".to_string(),
        observed_on: OBSERVED_ON.to_string(),
    })?)
}

fn acceptance() -> Result<CoachLaneAcceptance, Box<dyn std::error::Error>> {
    Ok(CoachLaneAcceptance::new(
        "census-ops",
        "jpx4.accepted_refusal",
        OBSERVED_ON,
    )?)
}

fn refusal(source: &str) -> Result<CoachLaneRefusal, Box<dyn std::error::Error>> {
    Ok(CoachLaneRefusal::new(
        SourceRef::id(source),
        "the association publishes no coach role columns",
        OBSERVED_ON,
    )?)
}

fn stage_outcome(records: usize) -> StageOutcome {
    StageOutcome {
        records,
        at: OBSERVED_ON.to_string(),
        errors: Vec::new(),
        notes: Vec::new(),
        disposition: census_crawl::CollectionDisposition::Complete,
        unfinished: Vec::new(),
    }
}

fn owed_contains(owed: &[String], item: &str) -> bool {
    owed.iter().any(|entry| entry == item)
}

#[test]
fn coach_lane_completed_source_without_rows_cannot_close_the_lane() -> TestResult {
    let outcome = TeamsSourceOutcome::Completed {
        outcome: stage_outcome(0),
        progress: Vec::new(),
    };
    check!(eq; outcome.coach_lane_stage(), CoachLaneStage::SuccessfulEmpty);
    let candidate = outcome
        .coach_lane_refusal("aia_directory", OBSERVED_ON)?
        .ok_or("a completed source run carries a refusal candidate")?;
    check!(candidate.reason().contains("0 records"));
    let recorded = pending(UsJurisdiction::Arizona, "aia_directory")?.record(
        outcome.coach_lane_stage(),
        Some(candidate),
        CoachLaneResolution::unresolved(OBSERVED_ON),
    )?;
    check!(
        !recorded.is_terminal(),
        "a source that completes with zero rows is not an accepted refusal"
    );
    check!(eq; recorded.status(), CoachLaneStatus::SuccessfulEmpty);
    let state = JurisdictionState {
        coach_lane: Some(recorded),
        ..JurisdictionState::default()
    };
    check!(coach_lane_owed(&state));
    let (owed, reasons) = owed_work(&state, &request(UsJurisdiction::Arizona)?);
    check!(owed_contains(&owed, "coach_lane"));
    check!(reasons
        .iter()
        .any(|reason| reason.contains("successful_empty")));
    Ok(())
}

#[test]
fn coach_lane_terminal_exhausted_and_interrupted_sources_stay_open() -> TestResult {
    let terminal = TeamsSourceOutcome::Terminal {
        message: "403 forbidden".to_string(),
        progress: Vec::new(),
    };
    check!(eq; terminal.coach_lane_stage(), CoachLaneStage::Blocked);
    let interrupted = TeamsSourceOutcome::Interrupted {
        attempts: Some(2),
        message: "endpoint shutdown".to_string(),
        progress: Vec::new(),
    };
    check!(eq; interrupted.coach_lane_stage(), CoachLaneStage::Blocked);
    let exhausted = TeamsSourceOutcome::Exhausted {
        attempts: 3,
        last_failure: "timeout".to_string(),
        progress: Vec::new(),
    };
    check!(eq; exhausted.coach_lane_stage(), CoachLaneStage::Exhausted);
    for outcome in [terminal, interrupted, exhausted] {
        let candidate = outcome.coach_lane_refusal("state_directory", OBSERVED_ON)?;
        check!(candidate.is_some());
        let recorded = pending(UsJurisdiction::Nevada, "state_directory")?.record(
            outcome.coach_lane_stage(),
            candidate,
            CoachLaneResolution::unresolved(OBSERVED_ON),
        )?;
        check!(
            !recorded.is_terminal(),
            "{} is a recorded outcome, not an accepted refusal",
            outcome.coach_lane_stage().slug()
        );
        check!(recorded.refusal().is_some());
        check!(recorded.acceptance().is_none());
    }
    Ok(())
}

#[test]
fn coach_lane_source_plan_refusal_is_recorded_and_never_accepted() -> TestResult {
    let plan = SourcePlan {
        sweepable: Vec::new(),
        refused: vec![RefusedSource {
            slug: "mhsaa_directory".to_string(),
            reason: "the directory refuses automated reads".to_string(),
            kind: RefusalKind::AccessBlocked,
        }],
        fingerprint: "fixture".to_string(),
    };
    let refusals = plan.coach_lane_refusals(OBSERVED_ON)?;
    check!(eq; refusals.len(), 1);
    check!(eq;
        refusals.first().map(CoachLaneRefusal::reason),
        Some("the directory refuses automated reads")
    );
    let reasonless = SourcePlan {
        refused: vec![RefusedSource {
            slug: "mhsaa_directory".to_string(),
            reason: String::new(),
            kind: RefusalKind::AccessBlocked,
        }],
        ..plan.clone()
    };
    check!(
        reasonless.coach_lane_refusals(OBSERVED_ON).is_err(),
        "a source-plan refusal without a reason cannot enter the coach lane"
    );
    let recorded = pending(UsJurisdiction::Michigan, "mhsaa_directory")?.record(
        CoachLaneStage::Blocked,
        refusals.into_iter().next(),
        CoachLaneResolution::unresolved(OBSERVED_ON),
    )?;
    check!(
        !recorded.is_terminal(),
        "a source plan that refuses a source is not an accepted refusal"
    );
    let mut state = JurisdictionState {
        plan: Some(plan),
        coach_lane: Some(recorded.clone()),
        ..JurisdictionState::default()
    };
    check!(coach_lane_owed(&state));
    let (owed, reasons) = owed_work(&state, &request(UsJurisdiction::Michigan)?);
    check!(owed_contains(&owed, "coach_lane"));
    check!(reasons
        .iter()
        .any(|reason| reason.contains("refused mhsaa_directory")));
    state.coach_lane = Some(recorded.accept_refusal(acceptance()?)?);
    check!(!coach_lane_owed(&state));
    let (owed, _) = owed_work(&state, &request(UsJurisdiction::Michigan)?);
    check!(!owed_contains(&owed, "coach_lane"));
    Ok(())
}

#[test]
fn coach_lane_contacts_stage_never_closes_the_lane() -> TestResult {
    let refusal = refusal("vt_directory")?;
    for (status, stage) in [
        (ContactStatus::Complete, CoachLaneStage::CompleteWithGaps),
        (ContactStatus::Partial, CoachLaneStage::CompleteWithGaps),
        (ContactStatus::Terminal, CoachLaneStage::Blocked),
        (ContactStatus::Interrupted, CoachLaneStage::Blocked),
        (ContactStatus::Exhausted, CoachLaneStage::Exhausted),
        (ContactStatus::HandlerFailed, CoachLaneStage::Failed),
    ] {
        check!(eq; status.coach_lane_stage(), stage);
        let recorded = pending(UsJurisdiction::Vermont, "vt_directory")?.record(
            status.coach_lane_stage(),
            Some(refusal.clone()),
            CoachLaneResolution::unresolved(OBSERVED_ON),
        )?;
        check!(
            !recorded.is_terminal(),
            "contacts stage {status:?} is coarse and cannot close the coach lane"
        );
    }
    Ok(())
}

#[test]
fn coach_lane_accepted_refusal_is_the_only_way_a_recorded_refusal_closes() -> TestResult {
    let recorded = pending(UsJurisdiction::Texas, "uil_directory")?.record(
        CoachLaneStage::Failed,
        Some(refusal("uil_directory")?),
        CoachLaneResolution::unresolved(OBSERVED_ON),
    )?;
    let mut state = JurisdictionState::default();
    check!(coach_lane_owed(&state));
    state.coach_lane = Some(recorded.clone());
    let (owed, reasons) = owed_work(&state, &request(UsJurisdiction::Texas)?);
    check!(owed_contains(&owed, "coach_lane"));
    check!(reasons.iter().any(|reason| reason.contains("failed")));
    check!(eq; recorded.acceptance().map(CoachLaneAcceptance::event), None);
    let closing = recorded.accept_refusal(acceptance()?)?;
    check!(closing.is_terminal());
    check!(eq; closing.status(), CoachLaneStatus::AcceptedRefusal);
    check!(eq;
        closing.acceptance().map(CoachLaneAcceptance::actor),
        Some("census-ops")
    );
    state.coach_lane = Some(closing);
    check!(!coach_lane_owed(&state));
    let (owed, _) = owed_work(&state, &request(UsJurisdiction::Texas)?);
    check!(!owed_contains(&owed, "coach_lane"));
    Ok(())
}

#[test]
fn coach_lane_armed_lane_closes_and_a_missing_lane_is_owed() -> TestResult {
    let mut state = JurisdictionState::default();
    check!(coach_lane_owed(&state));
    let (owed, reasons) = owed_work(&state, &request(UsJurisdiction::Ohio)?);
    check!(owed_contains(&owed, "coach_lane"));
    check!(reasons
        .iter()
        .any(|reason| reason.contains("no obligation recorded")));
    let evidence = CoachLaneEvidence::new(12, 19, DIGEST, OBSERVED_ON)?;
    let armed = pending(UsJurisdiction::Ohio, "arbiter_orgs")?
        .arm(evidence, CoachLaneResolution::unresolved(OBSERVED_ON))?;
    check!(armed.is_terminal());
    check!(eq; armed.status(), CoachLaneStatus::Armed);
    check!(eq; armed.evidence().map(CoachLaneEvidence::schools), Some(12));
    state.coach_lane = Some(armed);
    check!(!coach_lane_owed(&state));
    let (owed, _) = owed_work(&state, &request(UsJurisdiction::Ohio)?);
    check!(!owed_contains(&owed, "coach_lane"));
    Ok(())
}

#[test]
fn coach_lane_payload_round_trips_through_the_jurisdiction_state() -> TestResult {
    let armed = pending(UsJurisdiction::Iowa, "iahsaa_directory")?.arm(
        CoachLaneEvidence::new(7, 9, DIGEST, OBSERVED_ON)?,
        CoachLaneResolution::unresolved(OBSERVED_ON),
    )?;
    let state = JurisdictionState {
        coach_lane: Some(armed.clone()),
        ..JurisdictionState::default()
    };
    let encoded = serde_json::to_value(&state)?;
    let decoded: JurisdictionState = serde_json::from_value(encoded.clone())?;
    check!(eq; decoded.coach_lane, Some(armed));
    check!(!coach_lane_owed(&decoded));
    let mut absent = encoded;
    absent
        .as_object_mut()
        .ok_or("a jurisdiction state encodes as an object")?
        .remove("coach_lane");
    let decoded: JurisdictionState = serde_json::from_value(absent)?;
    check!(eq; decoded.coach_lane, None);
    check!(coach_lane_owed(&decoded));
    Ok(())
}
