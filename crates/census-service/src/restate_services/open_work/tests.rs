use super::*;
use crate::census::{MeetCensus, MeetSourceRows, StateProgress};
use crate::restate_services::jurisdiction::roster_stage_owed;
use crate::restate_services::results_arms::{ResultsSourceRows, ResultsStageOutcome};
use crate::restate_services::wire::{
    HistoryWindow, RefusedSource, SourcePlan, StageOutcome, TeamsFailure, TeamsStage,
};
use census_domain::model::{
    CanonicalSchool, CensusRun, ContactResearch, ContactResearchAttempt, ContactResearchOutcome,
    ContactResearchSubject, GradYear, RunManifest, SchoolMailboxPurpose,
};
use census_store::{Store, Table};
use sha2::{Digest, Sha256};

mod collection;
mod rosters;
type TestResult = Result<(), Box<dyn std::error::Error>>;

fn handler_error(error: restate_sdk::errors::HandlerError) -> Box<dyn std::error::Error> {
    format!("{error:?}").into()
}

fn historical_state() -> Result<JurisdictionState, Box<dyn std::error::Error>> {
    let mut state = JurisdictionState {
        history_window: Some(HistoryWindow::new(2024, 2026, "2026-10-07")?),
        plan: Some(SourcePlan {
            sweepable: vec!["milesplit".to_string()],
            refused: Vec::new(),
            fingerprint: "fixture".to_string(),
        }),
        ..JurisdictionState::default()
    };
    (2024..=2026).for_each(|year| {
        state.history.meets.insert(
            year,
            MeetCensus {
                sources: vec![MeetSourceRows {
                    slug: "milesplit".to_string(),
                    disposition: Disposition::Complete,
                    withheld: Some(0),
                    unresolved: Some(census_crawl::UnresolvedCounters { rows: 0, labels: 0 }),
                    ..MeetSourceRows::default()
                }],
                ..MeetCensus::default()
            },
        );
        state.history.results.insert(
            year,
            ResultsStageOutcome {
                required_sources: vec!["milesplit".to_string()],
                per_source: vec![result_source("milesplit", Disposition::Complete)],
                pending: Vec::new(),
            },
        );
    });
    Ok(state)
}

fn result_source(slug: &str, disposition: Disposition) -> ResultsSourceRows {
    ResultsSourceRows {
        slug: slug.to_string(),
        meets: 0,
        rows: Some(0),
        disposition,
        errors: 0,
        withheld: Some(0),
        notes: Vec::new(),
        unfinished: Vec::new(),
        unresolved: Some(census_crawl::UnresolvedCounters { rows: 0, labels: 0 }),
    }
}

#[test]
fn every_calendar_year_and_unresolved_result_locator_remains_owed() -> TestResult {
    let mut state = historical_state()?;
    check!(eq; history::status(&state, history::Kind::Meets).map_err(handler_error)?, Disposition::Complete);
    state
        .history
        .results
        .remove(&2025)
        .ok_or("missing result year")?;
    check!(eq; history::status(&state, history::Kind::Results).map_err(handler_error)?, Disposition::Unknown);
    let mut missing = ResultsStageOutcome {
        required_sources: vec!["milesplit".to_string()],
        per_source: vec![result_source("milesplit", Disposition::Complete)],
        pending: vec!["meet/date-unresolved".to_string()],
    };
    state.history.results.insert(2025, missing.clone());
    check!(eq; history::status(&state, history::Kind::Results).map_err(handler_error)?, Disposition::Unknown);
    missing.pending.clear();
    state.history.results.insert(2025, missing);
    check!(eq; history::status(&state, history::Kind::Results).map_err(handler_error)?, Disposition::Complete);
    state
        .history
        .meets
        .get_mut(&2024)
        .ok_or("missing meet year")?
        .sources
        .get_mut(0)
        .ok_or("missing meet source")?
        .disposition = Disposition::Unknown;
    check!(eq; history::status(&state, history::Kind::Meets).map_err(handler_error)?, Disposition::Unknown);
    Ok(())
}

#[test]
fn omitted_source_or_refusal_cannot_certify_the_registry_inventory() -> TestResult {
    let jurisdiction = UsJurisdiction::Wisconsin;
    let sweepable = census_crawl::applicability::applicable_sources(jurisdiction)
        .into_iter()
        .map(|source| source.slug.to_string())
        .collect();
    let mut entry = ReadJurisdiction {
        row: JurisdictionOpen {
            jurisdiction,
            identity: "fixture".to_string(),
            stages: JurisdictionStages::default(),
            unreadable: false,
        },
        state: JurisdictionState {
            plan: Some(SourcePlan {
                sweepable,
                refused: Vec::new(),
                fingerprint: "fixture".to_string(),
            }),
            ..JurisdictionState::default()
        },
    };
    check!(inventory::plan_covers(jurisdiction, &entry.state));
    let plan = entry.state.plan.as_mut().ok_or("missing source plan")?;
    let refused = plan.sweepable.pop().ok_or("empty registry inventory")?;
    check!(!inventory::plan_covers(jurisdiction, &entry.state));
    entry
        .state
        .plan
        .as_mut()
        .ok_or("missing source plan")?
        .refused
        .push(RefusedSource {
            slug: refused,
            reason: "unwired source".to_string(),
            kind: crate::restate_services::plan::RefusalKind::EngineeringGap,
        });
    check!(inventory::plan_covers(jurisdiction, &entry.state));
    check!(eq; stages_of(&entry.state, Disposition::Complete, Disposition::Complete, (Disposition::Unknown, 0)).map_err(handler_error)?.refused_sources, 1);
    check!(!stages_of(
        &entry.state,
        Disposition::Complete,
        Disposition::Complete,
        (Disposition::Unknown, 0)
    )
    .map_err(handler_error)?
    .terminal());
    Ok(())
}

fn school() -> Result<CanonicalSchool, Box<dyn std::error::Error>> {
    Ok(serde_json::from_value(
        serde_json::json!({ "id": "sch:wi-fixture", "name": "Fixture High School",
        "normalized_name": "fixture high school", "state": "WI", "co_op": false,
        "aliases": [], "source_identities": [], "evidence": [] }),
    )?)
}

fn research(school: &CanonicalSchool, subject: ContactResearchSubject) -> ContactResearch {
    ContactResearch {
        school: school.id.clone(),
        subject,
        school_year: SchoolYear::DEFAULT,
        outcome: ContactResearchOutcome::CompletedEmpty,
        attempts: vec![ContactResearchAttempt {
            locator: "https://fixture.example/staff".to_string(),
            acquired_at: "2026-10-07T12:00:00Z".to_string(),
            source_sha256: Some(format!(
                "{:x}",
                Sha256::digest(b"Captured staff directory: no matching professional contact")
            )),
            outcome: ContactResearchOutcome::CompletedEmpty,
            reason: "captured directory contains no matching professional contact".to_string(),
        }],
    }
}

fn bound_store(root: &std::path::Path) -> Result<Store, Box<dyn std::error::Error>> {
    let store = Store::open(root)?;
    store.bind_run(&RunManifest {
        store_identity: census_report::export::store_identity(&store)?,
        run: CensusRun::new(SchoolYear::DEFAULT, 1).ok_or("bad fixture run")?,
        cohort: GradYear::CO2027,
        jurisdictions: UsJurisdiction::CENSUS_SCOPE.to_vec(),
    })?;
    Ok(store)
}

fn school_owed(evidence: &StoreEvidence, school: &CanonicalSchool) -> usize {
    evidence
        .objects
        .iter()
        .filter(|row| {
            row.endpoint.starts_with(&format!("{}/contact/", school.id))
                && !row.disposition.is_complete()
        })
        .count()
}

#[test]
fn generic_offices_close_independently_and_missing_publication_still_blocks() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = bound_store(root.path())?;
    let mut school = school()?;
    school.contact_research = ContactResearch::programs()
        .into_iter()
        .map(|program| research(&school, ContactResearchSubject::Program(program)))
        .collect();
    store.append(Table::Schools, &school)?;
    let evidence = inspect_store(&store, SchoolYear::DEFAULT, Revision(1))?;
    check!(eq; school_owed(&evidence, &school), 2);
    check!(eq; evidence.contacts(UsJurisdiction::Wisconsin), Disposition::Unknown);
    school.contact_research.push(research(
        &school,
        ContactResearchSubject::SchoolMailbox(SchoolMailboxPurpose::SchoolOffice),
    ));
    store.append(Table::Schools, &school)?;
    check!(eq; school_owed(&inspect_store(&store, SchoolYear::DEFAULT, Revision(1))?, &school), 1);
    school.contact_research.push(research(
        &school,
        ContactResearchSubject::SchoolMailbox(SchoolMailboxPurpose::AthleticsOffice),
    ));
    store.append(Table::Schools, &school)?;
    let evidence = inspect_store(&store, SchoolYear::DEFAULT, Revision(1))?;
    check!(eq; school_owed(&evidence, &school), 0);
    check!(eq; evidence.contacts(UsJurisdiction::Wisconsin), Disposition::Complete);
    check!(evidence
        .objects
        .iter()
        .any(|row| row.endpoint == "publication/current-generation"
            && !row.disposition.is_complete()));
    Ok(())
}

#[test]
fn accepted_contact_attempt_does_not_close_an_unattempted_or_blocked_subject() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = bound_store(root.path())?;
    let mut school = school()?;
    let subject =
        ContactResearchSubject::Program(census_domain::model::CoachContactProgram::SchoolAthletics);
    let mut blocked = research(&school, subject);
    blocked.outcome = ContactResearchOutcome::Blocked;
    blocked
        .attempts
        .get_mut(0)
        .ok_or("missing contact attempt")?
        .outcome = ContactResearchOutcome::Blocked;
    school.contact_research.push(blocked);
    school.contact_research.push(research(
        &school,
        ContactResearchSubject::SchoolMailbox(SchoolMailboxPurpose::SchoolOffice),
    ));
    store.append(Table::Schools, &school)?;
    let evidence = inspect_store(&store, SchoolYear::DEFAULT, Revision(1))?;
    check!(eq; school_owed(&evidence, &school), 8);
    check!(evidence
        .objects
        .iter()
        .any(|row| row.endpoint.ends_with("/contact/SchoolAthletics")
            && row.disposition == Disposition::Blocked));
    check!(eq; evidence.contacts(UsJurisdiction::Wisconsin), Disposition::Unknown);
    Ok(())
}

#[test]
fn source_completion_does_not_close_a_different_sources_unknown_counters() -> TestResult {
    let mut state = historical_state()?;
    let outcome = state
        .history
        .results
        .get_mut(&2025)
        .ok_or("missing result year")?;
    let mut unknown = result_source("tfrrs", Disposition::Complete);
    unknown.unresolved = None;
    outcome.required_sources.push("tfrrs".to_string());
    outcome.per_source.push(unknown);
    check!(eq; history::source(&state, 2025, "milesplit", history::Kind::Results).map_err(handler_error)?.0, Disposition::Complete);
    check!(eq; history::source(&state, 2025, "tfrrs", history::Kind::Results).map_err(handler_error)?.0, Disposition::Partial);
    check!(!history::stage_complete(
        &state,
        2025,
        history::Kind::Results
    ));
    state
        .history
        .results
        .get_mut(&2025)
        .ok_or("missing result year")?
        .per_source
        .get_mut(1)
        .ok_or("missing result source")?
        .unresolved = Some(census_crawl::UnresolvedCounters { rows: 0, labels: 0 });
    check!(eq; history::source(&state, 2025, "tfrrs", history::Kind::Results).map_err(handler_error)?.0, Disposition::Complete);
    check!(history::stage_complete(
        &state,
        2025,
        history::Kind::Results
    ));
    Ok(())
}

fn independent_stages_completed(
    teams: TeamsStage,
) -> Result<JurisdictionState, Box<dyn std::error::Error>> {
    Ok(JurisdictionState {
        teams,
        rosters: Some(StateProgress {
            jurisdiction: UsJurisdiction::Wisconsin,
            rosters_total: 2,
            rosters_committed: 2,
            rosters_remaining: 0,
            rosters_skipped: 0,
            athletes: 11,
            class_of_2027: 7,
            class_of_2027_boys: 3,
            class_of_2027_girls: 4,
            errors: Vec::new(),
            blocked: false,
            blocked_skipped: 0,
            teams: 2,
        }),
        ..historical_state()?
    })
}

fn independent_stages(
    state: &JurisdictionState,
) -> Result<JurisdictionStages, Box<dyn std::error::Error>> {
    stages_of(
        state,
        Disposition::Complete,
        Disposition::Complete,
        (Disposition::Complete, 0),
    )
    .map_err(handler_error)
}

fn completed_teams() -> TeamsStage {
    TeamsStage::from_outcome(
        StageOutcome {
            records: 19,
            at: "2026-10-01".to_string(),
            errors: Vec::new(),
            notes: Vec::new(),
            disposition: Disposition::Complete,
            unfinished: Vec::new(),
        },
        "2026-10-01".to_string(),
    )
}

#[test]
fn roster_stage_owed_only_for_absent_or_nonterminal_progress() -> TestResult {
    check!(roster_stage_owed(&JurisdictionState::default()));
    for (remaining, blocked, blocked_skipped, expected_owed) in [
        (2, false, 0, true),
        (0, true, 0, true),
        (0, false, 2, true),
        (0, false, 0, false),
    ] {
        let mut state = independent_stages_completed(TeamsStage::Owed)?;
        let progress = state.rosters.as_mut().ok_or("missing roster fixture")?;
        progress.rosters_remaining = remaining;
        progress.blocked = blocked;
        progress.blocked_skipped = blocked_skipped;
        check!(eq; roster_stage_owed(&state), expected_owed);
    }
    Ok(())
}

#[test]
fn failed_teams_remain_owed_without_automatic_work() -> TestResult {
    let terminal = TeamsStage::Failed(TeamsFailure::ActionTerminal {
        at: "2026-10-01".to_string(),
        code: 503,
        message: "coach source unavailable".to_string(),
    });
    let partial = TeamsStage::from_outcome(
        StageOutcome {
            records: 19,
            at: "2026-10-01".to_string(),
            errors: vec!["coach source incomplete".to_string()],
            notes: Vec::new(),
            disposition: Disposition::Partial,
            unfinished: Vec::new(),
        },
        "2026-10-01".to_string(),
    );
    [terminal, partial].into_iter().try_for_each(|teams| {
        let state = independent_stages_completed(teams)?;
        let restored: JurisdictionState = serde_json::from_value(serde_json::to_value(state)?)?;
        let stages = independent_stages(&restored)?;
        check!(!restored.teams.is_owed());
        check!(!stages.terminal());
        check!(eq; stages.owing(), vec!["teams"]);
        check!(eq; crate::census::owed_jurisdictions(&[stages]), 1);
        Ok(())
    })
}

#[test]
fn completed_teams_allow_jurisdiction_completion() -> TestResult {
    let state = independent_stages_completed(completed_teams())?;
    let restored: JurisdictionState = serde_json::from_value(serde_json::to_value(state)?)?;
    let stages = independent_stages(&restored)?;
    check!(stages.terminal());
    check!(eq; stages.owing(), Vec::<&str>::new());
    check!(eq; crate::census::owed_jurisdictions(&[stages]), 0);
    Ok(())
}

#[test]
fn unfinished_rosters_keep_the_jurisdiction_nonterminal() -> TestResult {
    for (remaining, blocked, blocked_skipped, expected_owed) in
        [(2, false, 0, 2), (2, true, 2, 2), (0, true, 0, 0)]
    {
        let mut state = independent_stages_completed(completed_teams())?;
        let progress = state.rosters.as_mut().ok_or("missing roster fixture")?;
        progress.rosters_total = if remaining == 0 { 2 } else { 4 };
        progress.rosters_remaining = remaining;
        progress.blocked = blocked;
        progress.blocked_skipped = blocked_skipped;
        let stages = independent_stages(&state)?;
        check!(eq; stages.rosters, Disposition::Unknown);
        check!(eq; stages.owed_rosters, expected_owed);
        check!(!stages.terminal());
        check!(eq; crate::census::owed_jurisdictions(&[stages]), 1);
    }
    Ok(())
}

#[test]
fn results_failures_keep_the_jurisdiction_owed() -> TestResult {
    let clean = independent_stages_completed(completed_teams())?;
    let mut failed = clean.clone();
    failed
        .history
        .results
        .get_mut(&2026)
        .ok_or("missing results fixture")?
        .per_source
        .get_mut(0)
        .ok_or("missing results source")?
        .errors = 1;
    let mut absent = clean.clone();
    absent
        .history
        .results
        .remove(&2026)
        .ok_or("missing results fixture")?;
    for state in [&failed, &absent] {
        let stages = independent_stages(state)?;
        check!(!stages.results.is_complete());
        check!(eq; stages.owed_results, 1);
        check!(!stages.terminal());
        check!(eq; stages.owing(), vec!["results_history"]);
        check!(eq; crate::census::owed_jurisdictions(&[stages]), 1);
    }
    let stages = independent_stages(&clean)?;
    check!(stages.results.is_complete());
    check!(eq; stages.owed_results, 0);
    check!(stages.terminal());
    Ok(())
}
