use anyhow::Result;
use census_service::restate_services::JurisdictionState;
use serde_json::{json, Value};
use std::path::Path;

use super::artifacts::{oracle, write_json};
use super::ledger::Snapshot;
use super::scenario::{Faults, Run, SourceRepeat};

mod action;
mod faults;
mod parent;
mod source;

use parent::{admin_matches, independent_checks, reached_boundary, repeat_checks, tls_failure};
use source::{retained, source_checks};

pub struct Measurement {
    pub first: Run,
    pub repeat: Option<Run>,
    pub repeat_error: Option<String>,
    pub source_repeats: Vec<SourceRepeat>,
    pub request: Value,
    pub faults: Faults,
}

pub fn evaluate(root: &Path, measured: &Measurement, ledger: Option<&Snapshot>) -> Result<Value> {
    let first = &measured.first;
    let state: JurisdictionState = serde_json::from_value(first.state.clone())?;
    let persisted = admin_matches(first)?;
    let mut checks = source_checks(measured, &state, ledger)?;
    checks.extend(independent_checks(&state, persisted));
    checks.extend(faults::checks(&measured.faults, ledger)?);
    match &measured.repeat {
        Some(repeat) => {
            let repeated: JurisdictionState = serde_json::from_value(repeat.state.clone())?;
            checks.extend(repeat_checks(first, repeat, &state, &repeated)?);
            checks.push(oracle(
                "repeated_parent_admin_equals_ingress",
                admin_matches(repeat)?,
                json!({"state":repeat.state, "admin_state":repeat.admin_state}),
            ));
        }
        None => checks.extend([
            oracle(
                "final_report_refuses_teams_completion",
                false,
                json!({"repeat_error":measured.repeat_error}),
            ),
            oracle(
                "same_parent_key_no_fresh_teams_child_or_physical_request",
                false,
                json!({"repeat_error":measured.repeat_error}),
            ),
        ]),
    }
    checks.push(oracle(
        "failed_retains_actual_typed_source_failures",
        retained(first, &state)? && persisted,
        json!({"teams":state.teams, "admin_equals_ingress":persisted, "children":first.sources}),
    ));
    checks.push(oracle("real_isolated_TLS_failure_observed", tls_failure(first),
        json!({"physical_admissions":first.physical.len(), "observations":first.physical,
            "HTTP_status":null, "meaning":"TLS handshake admission and EOF, never an upstream HTTP 500; physical counts are not source reservation counts"})));
    let passed = checks
        .iter()
        .all(|check| check.get("status").and_then(Value::as_str) == Some("PASS"));
    let result = json!({"status":if passed {"PASS"} else {"BLOCKED_OR_UNPROVEN"}, "model":super::artifacts::MODEL,
        "oracles":checks, "reached_boundary":reached_boundary(&state), "first_state":first.state,
        "repeat_state":measured.repeat.as_ref().map(|repeat| &repeat.state), "repeat_error":measured.repeat_error,
        "physical_observations":measured.repeat.as_ref().map(|repeat| &repeat.physical),
        "source_repeats":measured.source_repeats, "source_faults":measured.faults, "cold_ledger":ledger,
        "native_retry_count_and_journal_run_completions_are_not_source_attempt_counters":true,
        "national_completeness":false, "fresh_positive_acquisition":"UNPROVEN; no cached parser lane is advertised as fresh acquisition"});
    write_json(&root.join("qualification-oracles.json"), &result)?;
    Ok(result)
}
