use athletic_rust_pipeline::result_verify::verify_results;
use serde_json::{json, Value};
use std::{fs, path::Path};
use tempfile::tempdir;

type TestResult = Result<(), Box<dyn std::error::Error>>;
fn digest(letter: char) -> String {
    (0..64).map(|_| letter).collect()
}

fn source() -> Value {
    json!({
        "source_key": "Roster:2",
        "sheet": "Roster",
        "excel_row": 2,
        "fields": {
            "Person First": "Ada",
            "Person Last": "Runner",
            "Schools Name": "Central High",
            "Address Mailing / Permanent City": "Austin",
            "Address Mailing / Permanent Region": "TX"
        }
    })
}

fn receipt(document: &str) -> Value {
    json!({
        "digest": document,
        "source_url": "https://example.test/source",
        "http_status": 200,
        "media_type": "application/json",
        "bytes": 1,
        "fetched_at_unix_ms": 1,
        "elapsed_ms": 1
    })
}

fn profile(document: &str, athlete_id: u64) -> Value {
    json!({
        "athlete_id": athlete_id,
        "profile": {
            "athlete_id": athlete_id,
            "profile_url": format!("https://www.athletic.net/athlete/{athlete_id}/track-and-field"),
            "name": {"value": "Ada Runner", "evidence": {"document": document, "locator": "/athlete/name"}},
            "teams": [{
                "team_id": 11,
                "name": {"value": "Central High", "evidence": {"document": document, "locator": "/team/name"}},
                "location": {"value": {"city_region": {"city": "Austin", "region": "TX"}}, "evidence": {"document": document, "locator": "/team/location"}},
                "seasons": [2024],
                "level": null
            }],
            "graduation_years": [],
            "grades": [],
            "sports": [{"state": "results_observed", "sport": "track_field", "count": 1}],
            "results": [{
                "result_id": 42,
                "sport": "track_field",
                "event_id": 100,
                "event_name": "100 Meter",
                "event_description": null,
                "event_type": null,
                "mark": "12.34",
                "units": "s",
                "season": 2024,
                "team_id": 11,
                "meet_id": 12,
                "meet_name": "Meet",
                "date": null,
                "wind": null,
                "timing": null,
                "personal_best": {"state": "unavailable"},
                "season_best": {"state": "unavailable"},
                "attribution": {"kind": "individual"},
                "short_code": null,
                "result_url": null,
                "evidence": {"document": document, "locator": "/results/0"}
            }],
            "issues": [],
            "documents": [document]
        },
        "responses": [receipt(document)],
        "operations": [],
        "failures": [],
        "complete": true
    })
}

fn assessment(athlete_id: u64, decision: &str, verified: Option<u64>) -> Value {
    json!({
        "decision": decision,
        "candidates": [{"athlete_id": athlete_id, "hard_eligible": false}],
        "search": {"Complete": {"evidence": digest('e')}},
        "verified": verified.map(|id| json!({"athlete_id": id}))
    })
}

fn accepted_row(method: &str) -> Value {
    let document = digest('a');
    json!({
        "source": source(),
        "report_digest": digest('b'),
        "report": {
            "revision": athletic_rust_pipeline::runtime::row_protocol::ROW_PROTOCOL_REVISION,
            "job": {"workbook": digest('c'), "snapshot": digest('d'), "source": "Roster:2"},
            "resolution": {"status": "accepted", "athlete_id": 7, "method": method},
            "assessment": digest('e'),
            "query_evidence": [],
            "profile_evidence": [digest('f')],
            "review": null,
            "issues": []
        },
        "assessment": assessment(7, "DeterministicAccepted", Some(7)),
        "profile_artifacts": [profile(&document, 7)],
        "performance_evidence": []
    })
}

fn write(path: &Path, rows: &[Value]) -> TestResult {
    let body = rows
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()?
        .join("\n");
    fs::write(path, format!("{body}\n"))?;
    Ok(())
}

#[test]
fn verifies_deterministic_positive_from_retained_evidence() -> TestResult {
    let directory = tempdir()?;
    let path = directory.path().join("detail.jsonl");
    write(&path, &[accepted_row("deterministic")])?;
    let report = verify_results(&path)?;
    assert_eq!(report.total_rows, 1);
    assert_eq!(report.accepted_rows, 1);
    assert_eq!(report.review_rows, 0);
    assert_eq!(report.no_match_rows, 0);
    assert_eq!(report.pending_rows, 0);
    Ok(())
}

#[test]
fn rejects_forged_selection_and_duplicate_source_keys() -> TestResult {
    let directory = tempdir()?;
    let forged_path = directory.path().join("forged.jsonl");
    let mut forged = accepted_row("deterministic");
    forged["report"]["resolution"]["athlete_id"] = json!(99);
    write_value(&forged_path, forged)?;
    assert!(verify_results(&forged_path).is_err());

    let duplicate_path = directory.path().join("duplicate.jsonl");
    write(
        &duplicate_path,
        &[accepted_row("deterministic"), accepted_row("deterministic")],
    )?;
    assert!(verify_results(&duplicate_path).is_err());
    Ok(())
}

#[test]
fn rejects_local_acceptance_without_review_artifact() -> TestResult {
    let directory = tempdir()?;
    let path = directory.path().join("local.jsonl");
    let mut row = accepted_row("local_review");
    row["assessment"] = assessment(7, "IdentityReview", None);
    write(&path, &[row])?;
    assert!(verify_results(&path).is_err());
    Ok(())
}

#[test]
fn preserves_review_without_assessment_as_non_acceptance() -> TestResult {
    let directory = tempdir()?;
    let path = directory.path().join("review.jsonl");
    let mut row = accepted_row("deterministic");
    row["report"]["resolution"] = json!({"status": "review_required"});
    row["report"]["assessment"] = Value::Null;
    row["assessment"] = Value::Null;
    write(&path, &[row])?;
    let report = verify_results(&path)?;
    assert_eq!(report.review_rows, 1);
    assert_eq!(report.accepted_rows, 0);
    Ok(())
}

#[test]
fn preserves_review_with_contradictory_profile_without_promotion() -> TestResult {
    let directory = tempdir()?;
    let path = directory.path().join("contradictory-review.jsonl");
    let document = digest('a');
    let mut row = accepted_row("deterministic");
    row["report"]["resolution"] = json!({"status": "review_required"});
    row["report"]["assessment"] = Value::Null;
    row["assessment"] = Value::Null;
    row["profile_artifacts"][0]["profile"]["issues"] = json!([{
        "code": "identity_conflict",
        "message": "synthetic contradiction",
        "evidence": {"document": document, "locator": "/identity"}
    }]);
    write(&path, &[row])?;
    let report = verify_results(&path)?;
    assert_eq!(report.review_rows, 1);
    assert_eq!(report.accepted_rows, 0);
    Ok(())
}

fn write_value(path: &Path, value: Value) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(path, format!("{}\n", serde_json::to_string(&value)?))?;
    Ok(())
}
