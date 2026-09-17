use athletic_rust_pipeline::{
    domain::{
        decision::{CandidateReason, Decision, SearchCompleteness},
        identity::AthleteId,
    },
    result_verify::verify_results,
    runtime::{acquisition::ProfileAcquisition, row_protocol::RowReport},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};
use tempfile::tempdir;

#[derive(Debug, Deserialize, Serialize)]
struct AssessmentFixture {
    decision: Decision,
    candidates: Vec<CandidateReason>,
    search: SearchCompleteness,
    #[serde(default)]
    verified: Option<VerifiedFixture>,
}

#[derive(Debug, Deserialize, Serialize)]
struct VerifiedFixture {
    athlete_id: AthleteId,
}

fn typed_digest<T: Serialize>(value: &T) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).expect("typed fixture"))
    )
}

fn normalize_row(mut row: Value) -> Value {
    if !row["assessment"].is_null() {
        let assessment: AssessmentFixture =
            serde_json::from_value(row["assessment"].clone()).expect("assessment fixture");
        row["report"]["assessment"] = json!(typed_digest(&assessment));
    } else {
        row["report"]["assessment"] = Value::Null;
    }
    let profile_digests = row["profile_artifacts"]
        .as_array()
        .expect("profile fixture array")
        .iter()
        .map(|value| {
            let profile: ProfileAcquisition =
                serde_json::from_value(value.clone()).expect("profile fixture");
            typed_digest(&profile)
        })
        .collect::<Vec<_>>();
    row["report"]["profile_evidence"] = json!(profile_digests);
    let report: RowReport = serde_json::from_value(row["report"].clone()).expect("report fixture");
    row["report_digest"] = json!(typed_digest(&report));
    row
}
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
    let raw = json!({
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
    });
    let typed: ProfileAcquisition = serde_json::from_value(raw).expect("valid profile fixture");
    serde_json::to_value(typed).expect("serialized profile fixture")
}

fn assessment(athlete_id: u64, decision: &str, verified: Option<u64>) -> Value {
    json!({
        "decision": decision,
        "candidates": [{
            "athlete_id": athlete_id,
            "exact_name": true,
            "matching_school": true,
            "location_corroborated": true,
            "hard_eligible": true,
            "participation_confirmed": true,
            "evidence_conflict": false,
            "evidence_strength": 100,
            "reasons": [],
            "evidence": [],
            "mailing_location_matches": true
        }],
        "search": {"Complete": {"evidence": digest('e')}},
        "verified": verified.map(|id| json!({"athlete_id": id}))
    })
}

fn accepted_row(method: &str) -> Value {
    let document = digest('a');
    let acquisition = profile(&document, 7);
    let performances = acquisition["profile"]["results"].clone();
    normalize_row(json!({
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
        "profile_artifacts": [acquisition],
        "performance_evidence": performances
    }))
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
    forged = normalize_row(forged);
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
    row = normalize_row(row);
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
    row = normalize_row(row);
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
    row = normalize_row(row);
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

#[test]
fn rejects_changed_artifacts_with_stale_declared_digests() -> TestResult {
    let directory = tempdir()?;
    // Given a complete accepted result, each independently addressed artifact is changed.
    let changes = [
        ("/report/issues", json!(["forged report note"])),
        (
            "/assessment/candidates/0/reasons",
            json!(["forged assessment reason"]),
        ),
        ("/profile_artifacts/0/responses/0/elapsed_ms", json!(2)),
    ];
    for (index, (pointer, replacement)) in changes.into_iter().enumerate() {
        let mut row = accepted_row("deterministic");
        *row.pointer_mut(pointer).expect("fixture artifact field") = replacement;
        let path = directory.path().join(format!("stale-{index}.jsonl"));
        write_value(&path, row)?;
        // When its retained body changes without its digest, verification must reject it.
        assert!(
            verify_results(&path).is_err(),
            "accepted stale digest for {pointer}"
        );
    }
    Ok(())
}

#[test]
fn rejects_performance_projection_that_differs_from_retained_results() -> TestResult {
    let directory = tempdir()?;
    let path = directory.path().join("performance-forgery.jsonl");
    // Given authentic profile artifacts, alter only the separately exported performance.
    let mut row = accepted_row("deterministic");
    row["performance_evidence"][0]["mark"] = json!("9.99");
    write_value(&path, row)?;
    // When verified, the unsupported performance claim must not be accepted.
    assert!(verify_results(&path).is_err());
    Ok(())
}

#[test]
fn rejects_duplicate_report_fields_with_ambiguous_meaning() -> TestResult {
    let directory = tempdir()?;
    let path = directory.path().join("duplicate-report.jsonl");
    // Given one valid report, a duplicate first report would disagree for first-wins consumers.
    let encoded = serde_json::to_string(&accepted_row("deterministic"))?;
    let remainder = encoded.strip_prefix('{').expect("fixture JSON object");
    fs::write(&path, format!("{{\"report\":null,{remainder}\n"))?;
    // When verified, duplicate evidence fields must be rejected rather than collapsed.
    assert!(verify_results(&path).is_err());
    Ok(())
}

fn local_selection(mut row: Value) -> Value {
    row["report"]["resolution"]["method"] = json!("local_review");
    row["report"]["review"] = json!({
        "outcome": "reviewed",
        "lane": "q5_5090",
        "verdict": {
            "decision": "select",
            "athlete_id": 7,
            "reason": "Selected supplied school evidence",
            "evidence": [{"document": digest('a'), "locator": "/team/name"}]
        },
        "request": digest('e'),
        "response": receipt(&digest('f')),
        "previous_responses": [],
        "retries": {"ownership": "not_attempted"}
    });
    normalize_row(row)
}

#[test]
fn rejects_indistinguishable_local_selection_with_rehashed_evidence() -> TestResult {
    let directory = tempdir()?;
    let path = directory.path().join("indistinguishable.jsonl");
    let mut row = accepted_row("deterministic");
    let second = profile(&digest('d'), 8);
    row["profile_artifacts"]
        .as_array_mut()
        .expect("profiles")
        .push(second.clone());
    row["performance_evidence"]
        .as_array_mut()
        .expect("results")
        .extend(
            second["profile"]["results"]
                .as_array()
                .expect("second results")
                .iter()
                .cloned(),
        );
    row["assessment"] = assessment(7, "IdentityReview", None);
    row["assessment"]["candidates"]
        .as_array_mut()
        .expect("candidates")
        .push(assessment(8, "IdentityReview", None)["candidates"][0].clone());
    // Rehash every changed artifact: stale-digest rejection cannot protect this case.
    write_value(&path, local_selection(row))?;
    assert!(verify_results(&path).is_err());
    Ok(())
}

#[test]
fn rejects_local_review_claim_for_deterministic_assessment() -> TestResult {
    let directory = tempdir()?;
    let path = directory.path().join("wrong-review-state.jsonl");
    write_value(&path, local_selection(accepted_row("deterministic")))?;
    assert!(verify_results(&path).is_err());
    Ok(())
}

#[test]
fn verifies_relay_identity_distinct_from_confirmed_member() -> TestResult {
    let directory = tempdir()?;
    let path = directory.path().join("relay-member.jsonl");
    let mut row = accepted_row("deterministic");
    row["profile_artifacts"][0]["profile"]["results"][0]["attribution"] =
        json!({"kind": "verified_relay_member", "relay_athlete_id": 99});
    row["performance_evidence"] = row["profile_artifacts"][0]["profile"]["results"].clone();
    write_value(&path, normalize_row(row))?;
    assert_eq!(verify_results(&path)?.accepted_rows, 1);
    Ok(())
}

#[test]
fn rejects_profile_identity_swap_with_matching_inner_url() -> TestResult {
    let directory = tempdir()?;
    let path = directory.path().join("swapped-profile.jsonl");
    let mut row = accepted_row("deterministic");
    row["profile_artifacts"][0]["profile"]["athlete_id"] = json!(8);
    row["profile_artifacts"][0]["profile"]["profile_url"] =
        json!("https://www.athletic.net/athlete/8/track-and-field");
    write_value(&path, normalize_row(row))?;
    assert!(verify_results(&path).is_err());
    Ok(())
}
