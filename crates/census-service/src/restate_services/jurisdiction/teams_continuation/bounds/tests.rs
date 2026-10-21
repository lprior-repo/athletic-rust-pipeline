use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn serialized_json_budget_counts_escaping_and_refuses_exactly_one_byte_over() -> TestResult {
    let mut budget = Budget::new();
    budget.value(&"\n").map_err(|error| format!("{error:?}"))?;
    check!(eq; budget.bytes, 4);
    budget.write_all(&vec![b'x'; MAX_BYTES - 4])?;
    check!(eq; budget.bytes, MAX_BYTES);
    check!(budget.write_all(b"x").is_err());
    check!(eq; budget.bytes, MAX_BYTES);
    Ok(())
}

#[test]
fn cumulative_diagnostics_refuse_before_rebuild_and_attempt_history_is_bounded() -> TestResult {
    let mut stage = crate::restate_services::jurisdiction::team_collection::empty_outcome(
        "2026-10-02",
        census_crawl::CollectionDisposition::Partial,
    );
    stage.notes = vec![String::new(); MAX_ROWS];
    let mut budget = Budget::new();
    budget.stage(&stage).map_err(|error| format!("{error:?}"))?;
    check!(eq; budget.rows, MAX_ROWS);
    stage.notes = vec![String::new()];
    check!(budget.stage(&stage).is_err());
    let source = TeamsSourceOutcome::Terminal {
        message: "bounded original failure".into(),
        progress: (1..=4)
            .map(|attempt| TeamsAttemptProgress::Unknown { attempt })
            .collect(),
    };
    check!(Budget::new().source(&source).is_err());
    Ok(())
}

#[test]
fn closed_source_inventory_refuses_twenty_two_without_cloning_state() -> TestResult {
    let request = JurisdictionRequest {
        jurisdiction: census_domain::UsJurisdiction::Ohio,
        season: census_domain::model::SchoolYear::new(2026).ok_or("invalid year")?,
        revision: census_domain::model::Revision(1),
        history: crate::restate_services::wire::HistoryWindow::cohort("2026-10-02")?,
        refresh: false,
        limit_per_state: None,
        concurrency: 1,
        observed_on: None,
        authorized_hosts: Vec::new(),
        source_parallelism: 1,
    };
    let mut state = JurisdictionState::default();
    state.plan = Some(crate::restate_services::wire::SourcePlan {
        sweepable: (0..MAX_SOURCES)
            .map(|value| format!("source-{value}"))
            .collect(),
        ..Default::default()
    });
    Budget::new()
        .parent(&state, &request)
        .map_err(|error| format!("{error:?}"))?;
    state
        .plan
        .as_mut()
        .ok_or("plan absent")?
        .sweepable
        .push("extra".into());
    check!(Budget::new().parent(&state, &request).is_err());
    Ok(())
}
