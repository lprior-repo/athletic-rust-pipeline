use super::*;
use census_domain::model::CoachLaneAcceptance;

fn completed_collection() -> Result<JurisdictionState, Box<dyn std::error::Error>> {
    let mut state = independent_stages_completed(completed_teams())?;
    let sweepable: Vec<String> =
        census_crawl::applicability::applicable_sources(UsJurisdiction::Wisconsin)
            .into_iter()
            .map(|source| source.slug.to_string())
            .collect();
    let meets = sweepable
        .iter()
        .filter(|slug| history::Kind::Meets.requires(slug))
        .map(|slug| MeetSourceRows {
            slug: slug.clone(),
            disposition: Disposition::Complete,
            withheld: Some(0),
            unresolved: Some(census_crawl::UnresolvedCounters { rows: 0, labels: 0 }),
            ..MeetSourceRows::default()
        })
        .collect();
    let required_sources: Vec<String> = sweepable
        .iter()
        .filter(|slug| history::Kind::Results.requires(slug))
        .cloned()
        .collect();
    let per_source = required_sources
        .iter()
        .map(|slug| result_source(slug, Disposition::Complete))
        .collect();
    state.plan = Some(SourcePlan {
        sweepable,
        refused: Vec::new(),
        fingerprint: "complete-fixture".to_string(),
    });
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
    check!(collection_is_complete(UsJurisdiction::Wisconsin, &state).map_err(handler_error)?);
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
            .find(|source| source.slug == "milesplit")
            .ok_or("missing MileSplit source")?
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
        .find(|source| source.slug == "milesplit")
        .ok_or("missing MileSplit source")?
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
    check!(collection_is_complete(UsJurisdiction::Wisconsin, &state).map_err(handler_error)?);
    let mut skipped = state.clone();
    skipped
        .rosters
        .as_mut()
        .ok_or("missing roster outcome")?
        .blocked_skipped = 1;
    check!(!collection_is_complete(UsJurisdiction::Wisconsin, &skipped).map_err(handler_error)?);
    let mut refused = state.clone();
    let plan = refused.plan.as_mut().ok_or("missing source plan")?;
    let slug = plan.sweepable.pop().ok_or("missing applicable source")?;
    plan.refused.push(RefusedSource {
        slug,
        reason: "unwired source".to_string(),
        kind: crate::restate_services::plan::RefusalKind::EngineeringGap,
    });
    check!(inventory::plan_covers(UsJurisdiction::Wisconsin, &refused));
    check!(!collection_is_complete(UsJurisdiction::Wisconsin, &refused).map_err(handler_error)?);
    let mut no_inventory = state.clone();
    no_inventory
        .plan
        .as_mut()
        .ok_or("missing source plan")?
        .sweepable
        .pop()
        .ok_or("missing applicable source")?;
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
