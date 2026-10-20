use super::*;
use crate::restate_services::open_work::source_objects::{class_counts, classify, keys};
use crate::restate_services::wire::{IngestState, SourceObjectInventory};

fn fixture_store() -> Result<(tempfile::TempDir, Store), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let store = bound_store(root.path())?;
    let mut batch = store.write_batch();
    batch.append_many(Table::Schools, &[school()?])?;
    batch.commit_once("populated", "fixture-populated")?;
    store.credit_observations("fixture/populated", "populated")?;
    store.write_batch().commit_once("empty", "fixture-empty")?;
    store.credit_observations("fixture/empty", "empty")?;
    Ok((root, store))
}

pub(super) fn enumerated_jurisdictions() -> Result<Vec<ReadJurisdiction>, Box<dyn std::error::Error>>
{
    UsJurisdiction::CENSUS_SCOPE
        .into_iter()
        .map(|jurisdiction| {
            let mut state = historical_state()?;
            state.plan = Some(SourcePlan {
                sweepable: census_crawl::applicability::applicable_sources(jurisdiction)
                    .into_iter()
                    .map(|source| source.slug.to_string())
                    .collect(),
                refused: Vec::new(),
                satisfied: Vec::new(),
                fingerprint: "fixture".to_string(),
            });
            Ok(ReadJurisdiction {
                row: JurisdictionOpen {
                    jurisdiction,
                    identity: jurisdiction.code().to_string(),
                    stages: JurisdictionStages::default(),
                    unreadable: false,
                    athleticlive_manifest: None,
                    athleticlive_manifest_reason: None,
                    teams_stage: None,
                    source_refusals: None,
                },
                state,
            })
        })
        .collect()
}

fn fixture_rows(
    store: &Store,
    keys: &[String],
) -> Result<Vec<SourceObjectOpen>, Box<dyn std::error::Error>> {
    keys.iter()
        .map(|endpoint| {
            let state = IngestState {
                endpoint: endpoint.clone(),
                total_observations: store.endpoint_credit(endpoint)?,
                windows_completed: 1,
                ..IngestState::default()
            };
            Ok(classify(endpoint, Some(&state)))
        })
        .collect()
}

fn fixture_reply(
    evidence: &StoreEvidence,
    rows: Vec<SourceObjectOpen>,
) -> Result<OpenWorkReply, Box<dyn std::error::Error>> {
    let inventory = class_counts(&rows).map_err(handler_error)?;
    reply(
        SchoolYear::DEFAULT,
        Revision(1),
        enumerated_jurisdictions()?,
        rows,
        evidence,
        inventory,
    )
    .map_err(handler_error)
}

#[test]
fn no_supplied_keys_measure_recorded_source_objects_like_the_per_key_path() -> TestResult {
    let (_root, store) = fixture_store()?;
    let mut evidence = inspect_store(&store, SchoolYear::DEFAULT, Revision(1))?;
    let rows = fixture_rows(&store, keys(&[], &evidence).map_err(handler_error)?)?;
    let report = fixture_reply(&evidence, rows)?;
    check!(eq; report.source_objects, Some(2));
    check!(eq; report.source_object_inventory, Some(SourceObjectInventory {
        objects: 2, accepted_observations: 1, accepted_nothing: 1, unreadable: 0,
    }));
    check!(eq; report.endpoints.len(), 2);
    check!(report
        .endpoints
        .iter()
        .any(|row| row.endpoint == "fixture/empty"
            && row.observations == 0
            && !row.unreadable
            && row.disposition == Disposition::Unknown));
    check!(report
        .endpoints
        .iter()
        .any(|row| row.endpoint == "fixture/populated"
            && row.observations == 1
            && !row.unreadable
            && row.disposition == Disposition::Unknown));
    let supplied = vec!["fixture/empty".to_string(), "fixture/populated".to_string()];
    let specific = fixture_reply(
        &evidence,
        fixture_rows(&store, keys(&supplied, &evidence).map_err(handler_error)?)?,
    )?;
    check!(eq; serde_json::to_value(report)?, serde_json::to_value(specific)?);
    let supplied = vec!["fixture/empty".to_string()];
    let specific = fixture_reply(
        &evidence,
        fixture_rows(&store, keys(&supplied, &evidence).map_err(handler_error)?)?,
    )?;
    check!(eq; specific.source_objects, Some(1));
    check!(eq; specific.source_object_inventory, Some(SourceObjectInventory {
        objects: 1, accepted_observations: 0, accepted_nothing: 1, unreadable: 0,
    }));
    evidence.bound = false;
    let unbound = fixture_reply(
        &evidence,
        fixture_rows(&store, keys(&[], &evidence).map_err(handler_error)?)?,
    )?;
    check!(eq; unbound.source_objects, None);
    check!(eq; unbound.source_object_inventory, None);
    Ok(())
}

#[test]
fn unreadable_or_foreign_ingest_state_is_not_counted_as_accepted_nothing() -> TestResult {
    let (_root, store) = fixture_store()?;
    let evidence = inspect_store(&store, SchoolYear::DEFAULT, Revision(1))?;
    let foreign = IngestState {
        endpoint: "another-object".to_string(),
        total_observations: 42,
        ..IngestState::default()
    };
    let rows = vec![
        classify("unreadable", None),
        classify("foreign", Some(&foreign)),
    ];
    let report = fixture_reply(&evidence, rows)?;
    check!(eq; report.source_objects, Some(2));
    check!(eq; report.source_object_inventory, Some(SourceObjectInventory {
        objects: 2, accepted_observations: 0, accepted_nothing: 0, unreadable: 2,
    }));
    check!(report
        .endpoints
        .iter()
        .all(|row| row.unreadable && row.disposition == Disposition::Unknown));
    Ok(())
}

#[test]
fn empty_inventory_is_measured_zero_but_incomplete_run_inventory_remains_unknown() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = bound_store(root.path())?;
    let evidence = inspect_store(&store, SchoolYear::DEFAULT, Revision(1))?;
    let rows = fixture_rows(&store, keys(&[], &evidence).map_err(handler_error)?)?;
    let report = fixture_reply(&evidence, rows)?;
    check!(eq; report.source_objects, Some(0));
    check!(eq; report.source_object_inventory, Some(SourceObjectInventory {
        objects: 0, accepted_observations: 0, accepted_nothing: 0, unreadable: 0,
    }));
    let mut jurisdictions = enumerated_jurisdictions()?;
    jurisdictions
        .first_mut()
        .ok_or("missing jurisdiction")?
        .state
        .plan = None;
    let report = reply(
        SchoolYear::DEFAULT,
        Revision(1),
        jurisdictions,
        Vec::new(),
        &evidence,
        class_counts(&[]).map_err(handler_error)?,
    )
    .map_err(handler_error)?;
    check!(eq; report.source_objects, None);
    Ok(())
}

#[test]
fn inventory_limit_refuses_instead_of_returning_a_partial_key_list() -> TestResult {
    let (_root, store) = fixture_store()?;
    check!(matches!(
        store.source_object_keys(1, MAX_ENDPOINTS),
        Err(census_store::StoreError::Refused { .. })
    ));
    check!(matches!(
        store.source_object_keys(MAX_OBJECTS, 1),
        Err(census_store::StoreError::Refused { .. })
    ));
    check!(eq; store.source_object_keys(2, MAX_ENDPOINTS)?, vec!["fixture/empty".to_string(), "fixture/populated".to_string()]);
    Ok(())
}

#[test]
fn supplied_keys_remain_measurable_when_the_recorded_inventory_cannot_be_enumerated() -> TestResult
{
    let (_root, store) = fixture_store()?;
    store.write_batch().commit_once("long", "fixture-long")?;
    let long_key = "x".repeat(MAX_ENDPOINTS.checked_add(1).ok_or("key size overflow")?);
    store.credit_observations(&long_key, "long")?;
    let evidence = inspect_store(&store, SchoolYear::DEFAULT, Revision(1))?;
    let supplied = vec!["fixture/empty".to_string()];
    check!(keys(&[], &evidence).is_err());
    let report = fixture_reply(
        &evidence,
        fixture_rows(&store, keys(&supplied, &evidence).map_err(handler_error)?)?,
    )?;
    check!(eq; report.source_objects, Some(1));
    check!(eq; report.source_object_inventory.map(|inventory| inventory.accepted_nothing), Some(1));
    Ok(())
}

#[test]
fn owed_team_source_readback_preserves_typed_failure_and_attempt_reasons() -> TestResult {
    use crate::restate_services::wire::{TeamsAttemptProgress, TeamsSourceOutcome};
    let outcome = StageOutcome {
        records: 7,
        at: "2026-10-12".to_string(),
        errors: vec!["schema mismatch for https://wi.milesplit.com/teams".to_string()],
        notes: Vec::new(),
        disposition: Disposition::Partial,
        unfinished: vec!["https://wi.milesplit.com/teams#row-8".to_string()],
    };
    let progress = vec![TeamsAttemptProgress::Transient {
        attempt: 3,
        outcome: Some(outcome.clone()),
        message: "http status 503 for https://wi.milesplit.com/teams".to_string(),
    }];
    let cases = [
        (
            TeamsSourceOutcome::Exhausted {
                attempts: 3,
                last_failure: "http status 503 for https://wi.milesplit.com/teams".to_string(),
                progress: progress.clone(),
            },
            Disposition::Exhausted,
            0,
        ),
        (
            TeamsSourceOutcome::Terminal {
                message: "source policy refuses acquisition".to_string(),
                progress: progress.clone(),
            },
            Disposition::Failed,
            0,
        ),
        (
            TeamsSourceOutcome::Interrupted {
                attempts: None,
                message: "invocation observation interrupted".to_string(),
                progress: progress.clone(),
            },
            Disposition::Partial,
            0,
        ),
        (
            TeamsSourceOutcome::Completed { outcome, progress },
            Disposition::Partial,
            7,
        ),
    ];
    for (outcome, disposition, observations) in cases {
        let expected = serde_json::to_value(&outcome)?;
        let mut row = obligation("WI/teams/milesplit".to_string(), Disposition::Unknown);
        inventory::project_team_outcome(&mut row, outcome).map_err(handler_error)?;
        check!(eq; row.disposition, disposition);
        check!(eq; row.observations, observations);
        let readback = serde_json::to_value(row)?;
        check!(eq; readback.get("teams_source"), Some(&expected));
        let restored: SourceObjectOpen = serde_json::from_value(readback)?;
        check!(eq; serde_json::to_value(restored.teams_source)?, expected);
    }
    Ok(())
}

#[test]
fn source_object_readback_decodes_journaled_rows_without_team_source_details() -> TestResult {
    let historical = serde_json::json!({
        "endpoint": "WI/teams/milesplit", "observations": 0, "windows": 0,
        "unreadable": false, "disposition": "exhausted"
    });
    let row: SourceObjectOpen = serde_json::from_value(historical.clone())?;
    check!(row.teams_source.is_none());
    check!(eq; serde_json::to_value(row)?, historical);
    Ok(())
}
