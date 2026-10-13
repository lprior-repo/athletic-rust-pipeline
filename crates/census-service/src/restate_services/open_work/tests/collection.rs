use super::*;
use census_domain::model::CoachLaneAcceptance;

use crate::restate_services::plan::{plan as planned, BrowserLaneState};
use crate::restate_services::wire::coach_lanes;

fn complete_meet_row(slug: &str) -> Result<MeetSourceRows, Box<dyn std::error::Error>> {
    let mut report = census_crawl::AdapterReport::new(slug, "history");
    report.rows = 1;
    report.disposition = Disposition::Complete;
    let census =
        crate::restate_services::meets_arms::report_census(slug, report).map_err(handler_error)?;
    census
        .sources
        .into_iter()
        .next()
        .ok_or("missing meet source row")
        .map_err(|error| error.into())
}
fn terminal_result_row(slug: &str) -> Result<ResultsSourceRows, Box<dyn std::error::Error>> {
    let mut report = census_crawl::AdapterReport::new(slug, "results");
    report.rows = 1;
    report.disposition = Disposition::Complete;
    crate::restate_services::results_arms::source_rows(slug, 1, report).map_err(handler_error)
}
fn delegated_terminal_row(
    slug: &str,
    acquired: &MeetSourceRows,
) -> Result<ResultsSourceRows, Box<dyn std::error::Error>> {
    let mut bound = crate::restate_services::results_arms::outcomes::delegated_source(slug);
    bound.rows = Some(acquired.rows);
    bound.disposition = acquired.disposition;
    bound.errors = u64::try_from(acquired.errors.len()).map_err(|_| "error count overflow")?;
    bound.notes = acquired.notes.clone();
    bound.notes.extend(acquired.errors.iter().cloned());
    bound.withheld = acquired.withheld;
    bound.unresolved = acquired.unresolved;
    bound.unfinished = acquired.unfinished.clone();
    let complete = acquired.disposition.is_complete()
        && acquired.unfinished.is_empty()
        && acquired.errors.is_empty()
        && acquired.withheld == Some(0)
        && acquired
            .unresolved
            .is_some_and(|value| value.rows == 0 && value.labels == 0);
    bound.resolution = complete.then_some(census_crawl::ResolutionCounters::default());
    Ok(bound)
}
fn completed_collection() -> Result<JurisdictionState, Box<dyn std::error::Error>> {
    let jurisdiction = UsJurisdiction::Wisconsin;
    let mut state = independent_stages_completed(completed_teams())?;
    let season = SchoolYear::new(2026).ok_or("invalid fixture school year")?;
    let lane = BrowserLaneState::Configured;
    let dispositions = planned(jurisdiction, lane);
    let fingerprint = crate::restate_services::plan::compute_plan_fingerprint(
        jurisdiction,
        season,
        census_reconcile::identity::Revision(1),
        lane,
    );
    let mut bound = SourcePlan::of(&dispositions, fingerprint);
    state.coach_lane = coach_lanes::bind(&mut bound, jurisdiction, season, 1, "2026-10-07")
        .map_err(|error| format!("binding coach lane failed: {error}"))?;
    state.plan = Some(bound);
    let sweepable = state
        .plan
        .as_ref()
        .ok_or("missing source plan")?
        .sweepable
        .clone();
    let mut meets = Vec::new();
    let mut required_sources = Vec::new();
    sweepable.iter().try_for_each(|slug| {
        if history::Kind::Meets.requires(slug.as_str()) {
            let row = complete_meet_row(slug.as_str())?;
            meets.try_reserve(1).map_err(|_| "meets allocation")?;
            meets.push(row);
        }
        if history::Kind::Results.requires(slug.as_str()) {
            required_sources
                .try_reserve(1)
                .map_err(|_| "required allocation")?;
            required_sources.push(slug.clone());
        }
        Ok::<(), Box<dyn std::error::Error>>(())
    })?;
    let mut per_source = Vec::new();
    required_sources.iter().try_for_each(|slug| {
        let row = if crate::restate_services::results_arms::arm_for(slug.as_str()).is_some() {
            terminal_result_row(slug.as_str())?
        } else {
            let acquired = meets
                .iter()
                .find(|source| source.slug.as_str() == slug.as_str())
                .ok_or("missing meets producer row")?;
            delegated_terminal_row(slug.as_str(), acquired)?
        };
        per_source
            .try_reserve(1)
            .map_err(|_| "results allocation")?;
        per_source.push(row);
        Ok::<(), Box<dyn std::error::Error>>(())
    })?;
    state.history_window = Some(HistoryWindow::new(2026, 2026, "2026-10-07")?);
    state.history.meets.clear();
    state.history.meets.insert(
        2026,
        MeetCensus {
            sources: meets,
            ..MeetCensus::default()
        },
    );
    state.history.results.clear();
    state.history.results.insert(
        2026,
        ResultsStageOutcome {
            required_sources,
            per_source,
            pending: Vec::new(),
        },
    );
    Ok(state)
}
#[test]
fn cen05_partial_source_work_cannot_become_a_completed_jurisdiction() -> TestResult {
    let state = completed_collection()?;
    check!(inventory::plan_covers(UsJurisdiction::Wisconsin, &state));
    check!(!collection_is_complete(UsJurisdiction::Wisconsin, &state).map_err(handler_error)?);
    let target = state
        .history
        .results
        .get(&2026)
        .ok_or("missing result year")?
        .required_sources
        .first()
        .ok_or("missing required source")?
        .clone();
    for disposition in [
        Disposition::Unknown,
        Disposition::Partial,
        Disposition::Blocked,
    ] {
        let mut partial = state.clone();
        partial
            .history
            .results
            .get_mut(&2026)
            .ok_or("missing result year")?
            .per_source
            .iter_mut()
            .find(|source| source.slug == target)
            .ok_or("missing required source")?
            .disposition = disposition;
        check!(!collection_is_complete(UsJurisdiction::Wisconsin, &partial).map_err(handler_error)?);
    }
    let mut unknown_rows = state.clone();
    unknown_rows
        .history
        .results
        .get_mut(&2026)
        .ok_or("missing result year")?
        .per_source
        .iter_mut()
        .find(|source| source.slug == target)
        .ok_or("missing required source")?
        .rows = None;
    check!(
        !collection_is_complete(UsJurisdiction::Wisconsin, &unknown_rows).map_err(handler_error)?
    );
    let mut owed = state.clone();
    owed.history
        .results
        .get_mut(&2026)
        .ok_or("missing result year")?
        .pending
        .push("meet/date-unresolved".to_string());
    check!(!collection_is_complete(UsJurisdiction::Wisconsin, &owed).map_err(handler_error)?);
    let mut missing_year = state.clone();
    missing_year.history.meets.clear();
    check!(
        !collection_is_complete(UsJurisdiction::Wisconsin, &missing_year).map_err(handler_error)?
    );
    Ok(())
}
#[test]
fn cen05_skipped_rosters_and_engineering_gaps_cannot_publish_completion() -> TestResult {
    let state = completed_collection()?;
    check!(inventory::plan_covers(UsJurisdiction::Wisconsin, &state));
    let plan = state.plan.as_ref().ok_or("missing source plan")?;
    let coach =
        coach_lanes::refusal_slug(UsJurisdiction::Wisconsin).ok_or("missing coach refusal")?;
    check!(plan.refused.iter().any(|entry| entry.slug == coach));
    check!(state.coach_lane.is_some());
    check!(!collection_is_complete(UsJurisdiction::Wisconsin, &state).map_err(handler_error)?);
    let mut skipped = state.clone();
    skipped
        .rosters
        .as_mut()
        .ok_or("missing roster outcome")?
        .blocked_skipped = 1;
    check!(!collection_is_complete(UsJurisdiction::Wisconsin, &skipped).map_err(handler_error)?);
    let mut no_inventory = state.clone();
    no_inventory
        .plan
        .as_mut()
        .ok_or("missing source plan")?
        .sweepable
        .pop()
        .ok_or("missing applicable source")?;
    let covered = inventory::plan_covers(UsJurisdiction::Wisconsin, &no_inventory);
    check!(!covered);
    check!(
        !collection_is_complete(UsJurisdiction::Wisconsin, &no_inventory).map_err(handler_error)?
    );
    Ok(())
}
#[test]
fn jpx41_an_accepted_coach_refusal_settles_the_plan_but_a_recorded_one_does_not() -> TestResult {
    let jurisdiction = UsJurisdiction::Oregon;
    let slug = crate::restate_services::wire::coach_lanes::refusal_slug(jurisdiction)
        .ok_or("oregon has no coach-lane refusal slug")?
        .to_string();
    let mut state = completed_collection()?;
    let season = SchoolYear::new(2026).ok_or("invalid school year")?;
    let mut bound = state.plan.clone().ok_or("missing source plan")?;
    state.coach_lane = crate::restate_services::wire::coach_lanes::bind(
        &mut bound,
        jurisdiction,
        season,
        1,
        "2026-10-12",
    )
    .map_err(|error| format!("binding the oregon coach lane failed: {error}"))?;
    bound.refused.clear();
    bound.refused.push(RefusedSource {
        slug: slug.clone(),
        reason: "unwired coach source".to_string(),
        kind: crate::restate_services::plan::RefusalKind::EngineeringGap,
    });
    check!(!refusals_settled(jurisdiction, &state, &bound));
    let lane = state
        .coach_lane
        .take()
        .ok_or("oregon coach lane was not bound")?;
    let acceptance = CoachLaneAcceptance::new("census-ops", "jpx4.accepted_refusal", "2026-10-12")
        .map_err(|error| format!("building the owner acceptance failed: {error}"))?;
    state.coach_lane = Some(
        lane.accept_refusal(acceptance)
            .map_err(|error| format!("accepting the oregon refusal failed: {error}"))?,
    );
    check!(refusals_settled(jurisdiction, &state, &bound));
    let mut unrelated = bound.clone();
    unrelated.refused.push(RefusedSource {
        slug: "an_unwired_source".to_string(),
        reason: "unwired source".to_string(),
        kind: crate::restate_services::plan::RefusalKind::EngineeringGap,
    });
    check!(!refusals_settled(jurisdiction, &state, &unrelated));
    check!(!refusals_settled(UsJurisdiction::Wisconsin, &state, &bound));
    Ok(())
}
#[test]
fn wi_delegated_results_reach_terminal_through_the_meets_producer() -> TestResult {
    let state = completed_collection()?;
    let outcome = state
        .history
        .results
        .get(&2026)
        .ok_or("missing result year")?
        .clone();
    check!(outcome.is_terminal());
    check!(eq; history::status(&state, history::Kind::Results).map_err(handler_error)?, Disposition::Complete);
    let delegated = outcome
        .required_sources
        .iter()
        .find(|slug| crate::restate_services::results_arms::arm_for(slug.as_str()).is_none())
        .ok_or("missing delegated lane")?
        .clone();
    let initial =
        crate::restate_services::results_arms::outcomes::delegated_source(delegated.as_str());
    check!(!initial.disposition.is_complete());
    check!(eq; initial.unfinished, vec![format!("{delegated}/acquisition-stage-outcome")]);
    let bound = outcome
        .per_source
        .iter()
        .find(|source| source.slug == delegated)
        .ok_or("missing bound producer row")?;
    let acquired = state
        .history
        .meets
        .get(&2026)
        .ok_or("missing meet year")?
        .sources
        .iter()
        .find(|source| source.slug == delegated)
        .ok_or("missing meets producer row")?;
    check!(eq; bound.disposition, Disposition::Complete);
    check!(eq; bound.rows, Some(acquired.rows));
    Ok(())
}
