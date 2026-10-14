use super::*;
use crate::restate_services::plan::RefusalKind;

fn entry() -> Result<ReadJurisdiction, Box<dyn std::error::Error>> {
    source_objects::enumerated_jurisdictions()?
        .into_iter()
        .next()
        .ok_or_else(|| "missing jurisdiction".into())
}

#[test]
fn diagnostics_preserve_actual_teams_stage_and_every_refusal_kind() -> TestResult {
    let stages = [
        TeamsStage::Owed,
        TeamsStage::Failed(TeamsFailure::ActionTerminal {
            at: "2026-10-14".to_string(),
            code: 500,
            message: "retained failure".to_string(),
        }),
        TeamsStage::from_outcome(
            StageOutcome {
                records: 7,
                at: "2026-10-14".to_string(),
                errors: Vec::new(),
                notes: Vec::new(),
                disposition: Disposition::Complete,
                unfinished: Vec::new(),
            },
            "2026-10-14".to_string(),
        ),
    ];
    for stage in stages {
        for kind in [
            RefusalKind::Unknown,
            RefusalKind::Terms,
            RefusalKind::EngineeringGap,
            RefusalKind::AccessBlocked,
        ] {
            let mut input = entry()?;
            input.state.teams = stage.clone();
            let refusals = vec![RefusedSource {
                slug: "fixture".to_string(),
                reason: "retained reason, not inferred".to_string(),
                kind,
            }];
            input.state.plan.as_mut().ok_or("missing plan")?.refused = refusals.clone();
            let row = diagnostic_row(input);
            check!(eq; serde_json::to_value(&row.teams_stage)?, serde_json::to_value(&stage)?);
            check!(eq; row.source_refusals, Some(refusals));
            let restored: JurisdictionOpen = serde_json::from_value(serde_json::to_value(&row)?)?;
            check!(eq; serde_json::to_value(restored)?, serde_json::to_value(row)?);
        }
    }
    Ok(())
}

#[test]
fn absent_plan_is_unmeasured_but_known_empty_refusals_are_measured() -> TestResult {
    let mut input = entry()?;
    input.state.plan = None;
    let absent = diagnostic_row(input);
    check!(absent.source_refusals.is_none());
    check!(absent.teams_stage.is_some());
    let empty = diagnostic_row(entry()?);
    check!(eq; empty.source_refusals, Some(Vec::new()));
    Ok(())
}

#[test]
fn unreadable_jurisdiction_never_fabricates_diagnostics() -> TestResult {
    let mut input = entry()?;
    input.row.unreadable = true;
    let row = diagnostic_row(input);
    check!(row.teams_stage.is_none());
    check!(row.source_refusals.is_none());
    Ok(())
}

#[test]
fn historical_response_absence_stays_unmeasured() -> TestResult {
    let mut value = serde_json::to_value(diagnostic_row(entry()?))?;
    let object = value.as_object_mut().ok_or("expected object")?;
    object.remove("teams_stage").ok_or("missing teams stage")?;
    object.remove("source_refusals").ok_or("missing refusals")?;
    let historical: JurisdictionOpen = serde_json::from_value(value.clone())?;
    check!(historical.teams_stage.is_none());
    check!(historical.source_refusals.is_none());
    check!(eq; serde_json::to_value(historical)?, value);
    Ok(())
}

#[test]
fn reply_projects_diagnostics_without_changing_completion() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = bound_store(root.path())?;
    let evidence = inspect_store(&store, SchoolYear::DEFAULT, Revision(1))?;
    let jurisdictions = source_objects::enumerated_jurisdictions()?;
    let report = reply(
        SchoolYear::DEFAULT,
        Revision(1),
        jurisdictions,
        Vec::new(),
        &evidence,
        SourceObjectInventory {
            objects: 0,
            accepted_observations: 0,
            accepted_nothing: 0,
            unreadable: 0,
        },
    )
    .map_err(handler_error)?;
    for row in report.jurisdictions {
        check!(matches!(row.teams_stage, Some(TeamsStage::Owed)));
        check!(eq; row.source_refusals, Some(Vec::new()));
        check!(eq; row.stages.teams, Disposition::Unknown);
    }
    Ok(())
}
