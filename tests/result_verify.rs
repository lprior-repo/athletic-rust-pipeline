use anyhow::Context;
use athletic_rust_pipeline::{
    domain::{
        decision::{CandidateReason, Decision, SearchCompleteness},
        identity::AthleteId,
    },
    result_verify::{verify_results, ResultVerificationReport},
    runtime::{
        acquisition::{ProfileAcquisition, ProfileProbe},
        row_protocol::{DiscoverySummary, RowReport},
    },
    store::ArtifactStore,
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

type TestResultReport = Result<ResultVerificationReport, Box<dyn std::error::Error>>;

fn verify(path: &Path) -> TestResultReport {
    let store_dir = path.parent().context("fixture path has no parent")?;
    let store = ArtifactStore::open(&store_dir.join("artifacts"))?;
    Ok(verify_results(path, &store)?)
}

fn seed_store(path: &Path, rows: &[Value]) -> TestResult {
    let root = path
        .parent()
        .context("fixture path has no parent")?
        .join("artifacts");
    let store = ArtifactStore::open(&root)?;
    rows.iter().try_for_each(|row| {
        if let Some(value) = row.get("discovery").filter(|value| !value.is_null()) {
            let typed: DiscoverySummary = serde_json::from_value(value.clone())?;
            put_serialized(
                &store,
                &typed,
                row["report"]["discovery"].as_str(),
                "discovery",
            )?;
        }
        if let Some(value) = row.get("assessment").filter(|value| !value.is_null()) {
            let typed: AssessmentFixture = serde_json::from_value(value.clone())?;
            put_serialized(
                &store,
                &typed,
                row["report"]["assessment"].as_str(),
                "assessment",
            )?;
        }
        row["identity_artifacts"]
            .as_array()
            .context("identity fixture array")?
            .iter()
            .try_for_each(|value| {
                let typed: ProfileProbe = serde_json::from_value(value.clone())?;
                put_serialized(&store, &typed, None, "probe")
            })?;
        row["profile_artifacts"]
            .as_array()
            .context("profile fixture array")?
            .iter()
            .try_for_each(|value| {
                let typed: ProfileAcquisition = serde_json::from_value(value.clone())?;
                put_serialized(&store, &typed, None, "profile")
            })?;
        if let Some(value) = row.get("report").filter(|value| !value.is_null()) {
            let typed: RowReport = serde_json::from_value(value.clone())?;
            put_serialized(&store, &typed, row["report_digest"].as_str(), "report")?;
        }
        Ok(())
    })
}

fn put_serialized<T: Serialize>(
    store: &ArtifactStore,
    value: &T,
    expected: Option<&str>,
    label: &str,
) -> TestResult {
    let actual = store.put_bytes(&serde_json::to_vec(value)?)?;
    if expected.is_some_and(|digest| digest != actual.as_str()) {
        return Err(format!("{label} fixture digest differs from its stored bytes").into());
    }
    Ok(())
}

fn normalize_row(mut row: Value) -> Value {
    let has_terminal_assessment = !row["assessment"].is_null();
    if !has_terminal_assessment && row["report"]["discovery"].is_null() {
        row["identity_artifacts"] = json!([]);
        row["profile_artifacts"] = json!([]);
        row["performance_evidence"] = json!([]);
        row["report"]["candidates"] = json!([]);
        let report: RowReport =
            serde_json::from_value(row["report"].clone()).expect("review report");
        row["report_digest"] = json!(typed_digest(&report));
        return row;
    }
    let profile_values = row["profile_artifacts"]
        .as_array()
        .expect("profile fixture array")
        .clone();
    let probes = profile_values
        .iter()
        .map(|value| {
            let acquisition: ProfileAcquisition =
                serde_json::from_value(value.clone()).expect("profile fixture");
            probe(acquisition.athlete_id.get(), value)
        })
        .collect::<Vec<_>>();
    let probe_digests = probes
        .iter()
        .map(|value| {
            let typed: ProfileProbe = serde_json::from_value(value.clone()).expect("probe fixture");
            typed_digest(&typed)
        })
        .collect::<Vec<_>>();
    row["identity_artifacts"] = json!(probes);
    let candidate_ids = profile_values
        .iter()
        .map(|value| value["athlete_id"].clone())
        .collect::<Vec<_>>();
    let discovery = json!({
        "job": row["report"]["job"],
        "candidate_ids": candidate_ids,
        "query_artifacts": [],
        "complete": true,
        "issues": []
    });
    let discovery_fixture: DiscoverySummary =
        serde_json::from_value(discovery.clone()).expect("discovery fixture");
    let discovery_digest = typed_digest(&discovery_fixture);
    row["discovery"] = serde_json::to_value(&discovery_fixture).expect("serialized discovery");
    row["report"]["discovery"] = json!(discovery_digest);
    if let Some(assessment) = row["assessment"].as_object_mut() {
        assessment["search"] = json!({"Complete": {"evidence": discovery_digest}});
        assessment["candidates"]
            .as_array_mut()
            .expect("assessment candidates")
            .iter_mut()
            .for_each(|candidate| {
                candidate["coverage"] = json!("complete");
                candidate["evidence"] = profile_values
                    .iter()
                    .find(|profile| profile["athlete_id"] == candidate["athlete_id"])
                    .map_or_else(
                        || json!([]),
                        |profile| profile["profile"]["documents"].clone(),
                    );
            });
        let assessment_fixture: AssessmentFixture =
            serde_json::from_value(row["assessment"].clone()).expect("assessment fixture");
        row["report"]["assessment"] = json!(typed_digest(&assessment_fixture));
    } else {
        row["report"]["assessment"] = Value::Null;
    }
    let candidates = profile_values
        .iter()
        .zip(probe_digests)
        .map(|(value, probe_digest)| {
            let profile: ProfileAcquisition =
                serde_json::from_value(value.clone()).expect("profile fixture");
            json!({"Complete": {
                "athlete_id": value["athlete_id"],
                "probe": probe_digest,
                "profile": typed_digest(&profile)
            }})
        })
        .collect::<Vec<_>>();
    row["report"]["candidates"] = json!(candidates);
    let report: RowReport = serde_json::from_value(row["report"].clone()).expect("report fixture");
    row["report_digest"] = json!(typed_digest(&report));
    row
}
fn operation(index: char) -> Value {
    json!({
        "operation": digest(index),
        "attempts": [digest(index)],
        "maximum_retries": 3,
        "observed_attempts": 1,
        "ownership": "sdk_controlled"
    })
}

fn html(document: &str, athlete_id: u64) -> Value {
    json!({
        "athlete_id": athlete_id,
        "profile_url": format!("https://www.athletic.net/athlete/{athlete_id}/track-and-field/all"),
        "cohort_witnesses": [],
        "tree_hints": [{
            "kind": "athlete",
            "id": athlete_id,
            "label": "Ada Runner",
            "evidence": {"document": document, "locator": "html/tree/0"}
        }],
        "identity_hints": [{
            "value": "Ada Runner",
            "evidence": {"document": document, "locator": "html/tree/0"}
        }],
        "issues": [],
        "document": document,
        "embedded_state": ["window.anetSiteAppParams={\"tree\":[]}"]
    })
}

fn probe(athlete_id: u64, acquisition: &Value) -> Value {
    let primary = acquisition["profile"]["documents"][0].clone();
    let secondary = digest('b');
    let html_document = digest('c');
    let mut first_profile = profile(primary.as_str().expect("profile document"), athlete_id);
    first_profile["profile"]["documents"] = json!([primary]);
    let mut second_profile = profile(&secondary, athlete_id);
    second_profile["profile"]["profile_url"] = json!(format!(
        "https://www.athletic.net/athlete/{athlete_id}/cross-country"
    ));
    second_profile["profile"]["sports"][0]["sport"] = json!("cross_country");
    second_profile["profile"]["results"][0]["sport"] = json!("cross_country");
    second_profile["profile"]["documents"] = json!([secondary]);
    let raw = json!({
        "athlete_id": athlete_id,
        "responses": [
            bio_receipt(primary.as_str().expect("primary document"), athlete_id, "tf"),
            bio_receipt(&secondary, athlete_id, "xc"),
            html_receipt(&html_document, athlete_id)
        ],
        "operations": [operation('a'), operation('b'), operation('c')],
        "failures": [],
        "profiles": [first_profile["profile"], second_profile["profile"]],
        "identities": [
            {"athlete_id": athlete_id, "sport": "track_field",
             "first": {"value": "Ada", "evidence": {"document": primary, "locator": "/athlete/FirstName"}},
             "last": {"value": "Runner", "evidence": {"document": primary, "locator": "/athlete/LastName"}}},
            {"athlete_id": athlete_id, "sport": "cross_country",
             "first": {"value": "Ada", "evidence": {"document": secondary, "locator": "/athlete/FirstName"}},
             "last": {"value": "Runner", "evidence": {"document": secondary, "locator": "/athlete/LastName"}}}
        ],
        "html": html(&html_document, athlete_id),
        "requests": [],
        "issues": [],
        "complete": true
    });
    let typed: ProfileProbe = serde_json::from_value(raw).expect("valid probe fixture");
    serde_json::to_value(typed).expect("serialized probe fixture")
}

fn html_receipt(document: &str, athlete_id: u64) -> Value {
    json!({
        "digest": document,
        "source_url": format!("https://www.athletic.net/athlete/{athlete_id}/track-and-field/all"),
        "http_status": 200,
        "media_type": "text/html",
        "bytes": 1,
        "fetched_at_unix_ms": 1,
        "elapsed_ms": 1
    })
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

fn bio_receipt(document: &str, athlete_id: u64, sport: &str) -> Value {
    let mut value = receipt(document);
    value["source_url"] = json!(format!(
        "https://www.athletic.net/api/v1/AthleteBio/GetAthleteBioData?athleteId={athlete_id}&sport={sport}&level=0"
    ));
    value
}

fn profile(document: &str, athlete_id: u64) -> Value {
    let documents = [document.to_owned(), digest('b'), digest('c'), digest('d')];
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
            "documents": documents
        },
        "responses": [bio_receipt(document, athlete_id, "tf"),
            bio_receipt(&documents[1], athlete_id, "xc"), html_receipt(&documents[2], athlete_id),
            receipt(&documents[3]), receipt(&documents[3])],
        "operations": [operation('a'), operation('b'), operation('c'),
            operation('d'), operation('e')],
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
        "discovery": null,
        "identity_artifacts": [],
        "report_digest": digest('b'),
        "report": {
            "revision": athletic_rust_pipeline::runtime::row_protocol::ROW_PROTOCOL_REVISION,
            "job": {"workbook": digest('c'), "snapshot": digest('d'), "source": "Roster:2"},
            "resolution": {"status": "accepted", "athlete_id": 7, "method": method},
            "discovery": digest('g'),
            "candidates": [],
            "assessment": digest('e'),
            "query_evidence": [],
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
    seed_store(path, rows)
}

#[test]
fn verifies_deterministic_positive_from_retained_evidence() -> TestResult {
    let directory = tempdir()?;
    let path = directory.path().join("detail.jsonl");
    write(&path, &[accepted_row("deterministic")])?;
    let report = verify(&path)?;
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
    let trusted = accepted_row("deterministic");
    seed_store(&forged_path, std::slice::from_ref(&trusted))?;
    let mut forged = trusted;
    forged["report"]["resolution"]["athlete_id"] = json!(99);
    forged = normalize_row(forged);
    write_value(&forged_path, forged)?;
    assert!(verify(&forged_path).is_err());

    let duplicate_path = directory.path().join("duplicate.jsonl");
    write(
        &duplicate_path,
        &[accepted_row("deterministic"), accepted_row("deterministic")],
    )?;
    assert!(verify(&duplicate_path).is_err());
    Ok(())
}
#[test]
fn rejects_local_acceptance_without_review_artifact() -> TestResult {
    let directory = tempdir()?;
    let path = directory.path().join("local.jsonl");
    let trusted = accepted_row("local_review");
    seed_store(&path, std::slice::from_ref(&trusted))?;
    let mut row = trusted;
    row["assessment"] = assessment(7, "IdentityReview", None);
    row = normalize_row(row);
    write_value(&path, row)?;
    assert!(verify(&path).is_err());
    Ok(())
}

#[test]
fn preserves_source_validation_review_without_assessment() -> TestResult {
    let directory = tempdir()?;
    let path = directory.path().join("review.jsonl");
    let mut row = accepted_row("deterministic");
    row["source"]["fields"]["Person First"] = json!("");
    row["discovery"] = Value::Null;
    row["report"]["discovery"] = Value::Null;
    row["report"]["candidates"] = json!([]);
    row["report"]["query_evidence"] = json!([]);
    row["report"]["review"] = Value::Null;
    row["report"]["issues"] = json!(["source identity is incomplete"]);
    row["report"]["resolution"] = json!({"status": "review_required"});
    row["report"]["assessment"] = Value::Null;
    row["assessment"] = Value::Null;
    row = normalize_row(row);
    write(&path, &[row])?;
    let report = verify(&path)?;
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
    row["assessment"] = assessment(7, "IdentityReview", None);
    row["profile_artifacts"][0]["profile"]["issues"] = json!([{
        "code": "identity_conflict",
        "message": "synthetic contradiction",
        "evidence": {"document": document, "locator": "/identity"}
    }]);
    row = normalize_row(row);
    write(&path, &[row])?;
    let report = verify(&path)?;
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
    let changes = [
        ("/report/issues", json!(["forged report note"])),
        (
            "/assessment/candidates/0/reasons",
            json!(["forged assessment reason"]),
        ),
        ("/profile_artifacts/0/responses/0/elapsed_ms", json!(2)),
    ];
    for (index, (pointer, replacement)) in changes.into_iter().enumerate() {
        let trusted = accepted_row("deterministic");
        let path = directory.path().join(format!("stale-{index}.jsonl"));
        seed_store(&path, std::slice::from_ref(&trusted))?;
        let mut row = trusted;
        *row.pointer_mut(pointer).expect("fixture artifact field") = replacement;
        write_value(&path, row)?;
        assert!(
            verify(&path).is_err(),
            "accepted stale digest for {pointer}"
        );
    }
    Ok(())
}

#[test]
fn rejects_performance_projection_that_differs_from_retained_results() -> TestResult {
    let directory = tempdir()?;
    let path = directory.path().join("performance-forgery.jsonl");
    let trusted = accepted_row("deterministic");
    seed_store(&path, std::slice::from_ref(&trusted))?;
    let mut row = trusted;
    row["performance_evidence"][0]["mark"] = json!("9.99");
    write_value(&path, row)?;
    assert!(verify(&path).is_err());
    Ok(())
}

#[test]
fn rejects_duplicate_report_fields_with_ambiguous_meaning() -> TestResult {
    let directory = tempdir()?;
    let path = directory.path().join("duplicate-report.jsonl");
    let trusted = accepted_row("deterministic");
    seed_store(&path, std::slice::from_ref(&trusted))?;
    let encoded = serde_json::to_string(&trusted)?;
    let remainder = encoded.strip_prefix('{').expect("fixture JSON object");
    fs::write(&path, format!("{{\"report\":null,{remainder}\n"))?;
    // When verified, duplicate evidence fields must be rejected rather than collapsed.
    assert!(verify(&path).is_err());
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
    let trusted = accepted_row("deterministic");
    seed_store(&path, std::slice::from_ref(&trusted))?;
    let mut row = trusted;
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
    assert!(verify(&path).is_err());
    Ok(())
}

#[test]
fn rejects_local_review_claim_for_deterministic_assessment() -> TestResult {
    let directory = tempdir()?;
    let path = directory.path().join("wrong-review-state.jsonl");
    let trusted = accepted_row("deterministic");
    seed_store(&path, std::slice::from_ref(&trusted))?;
    write_value(&path, local_selection(trusted))?;
    assert!(verify(&path).is_err());
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
    write(&path, &[normalize_row(row)])?;
    assert_eq!(verify(&path)?.accepted_rows, 1);
    Ok(())
}

#[test]
fn rejects_profile_identity_swap_with_matching_inner_url() -> TestResult {
    let directory = tempdir()?;
    let path = directory.path().join("swapped-profile.jsonl");
    let trusted = accepted_row("deterministic");
    seed_store(&path, std::slice::from_ref(&trusted))?;
    let mut row = trusted;
    row["profile_artifacts"][0]["profile"]["athlete_id"] = json!(8);
    row["profile_artifacts"][0]["profile"]["profile_url"] =
        json!("https://www.athletic.net/athlete/8/track-and-field");
    write_value(&path, normalize_row(row))?;
    assert!(verify(&path).is_err());
    Ok(())
}
