use super::*;

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
