use super::*;
use crate::restate_services::wire::AthleticLiveObligation;

fn manifest_report(
    work: Option<AthleticLiveObligation>,
) -> Result<OpenWorkReply, Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let store = bound_store(root.path())?;
    let evidence = inspect_store(&store, SchoolYear::DEFAULT, Revision(1))?;
    let mut jurisdictions = super::source_objects::enumerated_jurisdictions()?;
    for entry in &mut jurisdictions {
        entry.row.stages = JurisdictionStages {
            teams: Disposition::Complete,
            rosters: Disposition::Complete,
            meets: Disposition::Complete,
            results: Disposition::Complete,
            contacts: Disposition::Complete,
            publication: Disposition::Complete,
            ..JurisdictionStages::default()
        };
    }
    jurisdictions
        .first_mut()
        .ok_or("missing jurisdiction")?
        .state
        .athleticlive = work;
    reply(
        SchoolYear::DEFAULT,
        Revision(1),
        jurisdictions,
        Vec::new(),
        &evidence,
        crate::restate_services::open_work::source_objects::class_counts(&[])
            .map_err(handler_error)?,
    )
    .map_err(handler_error)
}

#[test]
fn refused_manifest_remains_owed_when_existing_stages_are_complete() -> TestResult {
    let refusal = AthleticLiveObligation::Refused {
        reason: "manifest not supplied".to_string(),
    };
    let report = manifest_report(Some(refusal.clone()))?;
    check!(eq; report.jurisdiction_sweeps, Some(1));
    check!(eq; report.source_objects, Some(1));
    let row = report.jurisdictions.first().ok_or("missing jurisdiction")?;
    check!(row.stages.terminal());
    check!(eq; row.athleticlive_manifest_reason.as_deref(), Some("athleticlive_manifest: refused: manifest not supplied"));
    check!(eq; serde_json::to_value(&row.athleticlive_manifest)?, serde_json::to_value(refusal)?);
    check!(eq; report.endpoints.first().ok_or("missing manifest endpoint")?.endpoint.as_str(), "AL/athleticlive_manifest");
    Ok(())
}

#[test]
fn partial_manifest_projects_exact_current_progress_and_retained_locators() -> TestResult {
    let work = AthleticLiveObligation::Finished {
        digest: "fixture-sha256".to_string(),
        attempt: 2,
        outcome: StageOutcome {
            records: 17,
            at: "2026-10-12".to_string(),
            errors: vec!["capture/result-2 unreadable".to_string()],
            notes: vec!["result-1 retained".to_string()],
            disposition: Disposition::Partial,
            unfinished: vec!["capture/result-2".to_string()],
        },
    };
    let report = manifest_report(Some(work.clone()))?;
    check!(eq; report.jurisdiction_sweeps, Some(1));
    check!(eq; report.source_objects, Some(1));
    let row = report.jurisdictions.first().ok_or("missing jurisdiction")?;
    check!(eq; serde_json::to_value(&row.athleticlive_manifest)?, serde_json::to_value(&work)?);
    check!(eq; row.athleticlive_manifest_reason.as_deref(), Some(work.reason().as_str()));
    let endpoint = report
        .endpoints
        .first()
        .ok_or("missing manifest endpoint")?;
    check!(eq; endpoint.observations, 17);
    check!(eq; endpoint.disposition, Disposition::Partial);
    Ok(())
}

#[test]
fn terminal_manifest_is_not_projected_as_owed_but_nominal_complete_with_errors_is() -> TestResult {
    let clean = AthleticLiveObligation::Finished {
        digest: "fixture-sha256".to_string(),
        attempt: 1,
        outcome: StageOutcome {
            records: 17,
            at: "2026-10-12".to_string(),
            errors: Vec::new(),
            notes: Vec::new(),
            disposition: Disposition::Complete,
            unfinished: Vec::new(),
        },
    };
    let report = manifest_report(Some(clean.clone()))?;
    check!(eq; report.jurisdiction_sweeps, Some(0));
    check!(eq; report.source_objects, Some(0));
    let row = report.jurisdictions.first().ok_or("missing jurisdiction")?;
    check!(eq; serde_json::to_value(&row.athleticlive_manifest)?, serde_json::Value::Null);
    check!(eq; row.athleticlive_manifest_reason, None);
    check!(eq; report.endpoints.len(), 0);
    let mut failed = clean;
    if let AthleticLiveObligation::Finished { outcome, .. } = &mut failed {
        outcome
            .errors
            .push("capture/result-2 unreadable".to_string());
    }
    let report = manifest_report(Some(failed))?;
    check!(eq; report.jurisdiction_sweeps, Some(1));
    check!(eq; report.source_objects, Some(1));
    check!(eq; report.endpoints.first().ok_or("missing manifest endpoint")?.disposition, Disposition::Partial);
    Ok(())
}

#[test]
fn legacy_open_work_reply_deserializes_without_additive_inventory_or_manifest_fields() -> TestResult
{
    let reply = manifest_report(None)?;
    let mut json = serde_json::to_value(reply)?;
    json.as_object_mut()
        .ok_or("missing reply object")?
        .remove("source_object_inventory");
    let legacy: OpenWorkReply = serde_json::from_value(json)?;
    check!(eq; legacy.source_object_inventory, None);
    check!(eq; legacy.jurisdictions.first().ok_or("missing jurisdiction")?.athleticlive_manifest_reason, None);
    check!(eq; legacy.source_objects, Some(0));
    Ok(())
}
