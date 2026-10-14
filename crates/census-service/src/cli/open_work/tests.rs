use super::*;
use census_domain::UsJurisdiction;
use census_service::census::JurisdictionStages;
use census_service::restate_services::{RefusedSource, TeamsFailure, TeamsStage};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn row() -> JurisdictionOpen {
    JurisdictionOpen {
        jurisdiction: UsJurisdiction::Wisconsin,
        identity: "fixture".to_string(),
        stages: JurisdictionStages::default(),
        unreadable: false,
        teams_stage: None,
        source_refusals: None,
        athleticlive_manifest: None,
        athleticlive_manifest_reason: None,
    }
}

fn rendered(row: &JurisdictionOpen) -> Result<String, Box<dyn std::error::Error>> {
    let mut bytes = Vec::new();
    print_diagnostics(&mut bytes, row)?;
    Ok(String::from_utf8(bytes)?)
}

#[test]
fn missing_and_measured_empty_diagnostics_stay_distinct() -> TestResult {
    let mut row = row();
    check!(eq; rendered(&row)?, "  WI teams stage: unmeasured\n  WI source refusals: unmeasured\n");
    row.teams_stage = Some(TeamsStage::Owed);
    row.source_refusals = Some(Vec::new());
    check!(eq; rendered(&row)?, "  WI teams stage: {\"status\":\"owed\"}\n  WI source refusals: 0\n");
    row.unreadable = true;
    check!(eq; rendered(&row)?, "");
    Ok(())
}

#[test]
fn formatter_preserves_canonical_failure_and_refusal_payloads() -> TestResult {
    let mut row = row();
    let stage = TeamsStage::Failed(TeamsFailure::ActionTerminal {
        at: "2026-10-14".to_string(),
        code: 500,
        message: "retained failure\nwith detail".to_string(),
    });
    let refusal: RefusedSource = serde_json::from_value(serde_json::json!({
        "slug": "fixture", "reason": "terms\nretain exact reason", "kind": "terms"
    }))?;
    let expected = format!(
        "  WI teams stage: {}\n  WI source refusals: 1\n    {}\n",
        serde_json::to_string(&stage)?,
        serde_json::to_string(&refusal)?
    );
    row.teams_stage = Some(stage);
    row.source_refusals = Some(vec![refusal]);
    check!(eq; rendered(&row)?, expected);
    Ok(())
}
