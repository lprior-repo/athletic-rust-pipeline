#![forbid(unsafe_code)]

use anyhow::{Context, Result};
use athletic_rust_pipeline::{
    domain::{
        candidate::CandidateEvidence,
        decision::{self, SearchCompleteness},
        identity::EvidenceDigest,
    },
    result_verify::{verify_results, ResultVerificationReport},
    runtime::{
        acquisition::{ProfileAcquisition, ProfileProbe, QueryEvidence, QueryPage},
        row_protocol::{DiscoverySummary, RowReport, ROW_PROTOCOL_REVISION},
        run_protocol::SourceSnapshot,
    },
    search::{parse_page, query_plan, SearchPage},
    store::ArtifactStore,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};
use tempfile::tempdir;

const ORIGIN: &str = "http://127.0.0.1:18081/";
const NATIVE_ACCEPTED: &[u8] =
    include_bytes!("../fuzz/fixtures/retained_results_jsonl/seed-valid-accepted.json");
const NATIVE_CONTRADICTION: &[u8] =
    include_bytes!("../fuzz/fixtures/retained_results_jsonl/seed-hard-contradiction.json");
const RAW_TF: &[u8] = include_bytes!(
    "../fuzz/fixtures/retained_results_jsonl/raw/37b72516909fc95821215c5f9ec1ce40e1d696c613dad8f96a22e140ca477560"
);
const RAW_XC: &[u8] = include_bytes!(
    "../fuzz/fixtures/retained_results_jsonl/raw/2fc43c65625879c99e34adff06876c86bcfd685073ea9d23cf84b0beb798f531"
);
const RAW_HTML: &[u8] = include_bytes!(
    "../fuzz/fixtures/retained_results_jsonl/raw/6bfbe82b524a8a7a14fe4d286e22b154d99f33baf77e43e96cba03ebd6cf1ae3"
);
const OLD_TF: &str = "37b72516909fc95821215c5f9ec1ce40e1d696c613dad8f96a22e140ca477560";
const OLD_XC: &str = "2fc43c65625879c99e34adff06876c86bcfd685073ea9d23cf84b0beb798f531";
const OLD_HTML: &str = "6bfbe82b524a8a7a14fe4d286e22b154d99f33baf77e43e96cba03ebd6cf1ae3";
const OLD_TEAM: &str = "a5d54ee54f8f147ff02fea9aafd3c7836e57c77e4cce12cb191836aa499a1135";

#[derive(Debug, Deserialize, Serialize)]
struct AssessmentFixture {
    decision: decision::Decision,
    candidates: Vec<decision::CandidateReason>,
    search: SearchCompleteness,
    #[serde(default)]
    verified: Option<VerifiedFixture>,
}

#[derive(Debug, Deserialize, Serialize)]
struct VerifiedFixture {
    athlete_id: athletic_rust_pipeline::domain::identity::AthleteId,
}

type TestResult = Result<(), Box<dyn std::error::Error>>;
type TestResultReport = Result<ResultVerificationReport, Box<dyn std::error::Error>>;

fn typed_digest<T: Serialize>(value: &T) -> Result<String> {
    let bytes = serde_json::to_vec(value)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn verify(path: &Path) -> TestResultReport {
    let store_dir = path.parent().context("fixture path has no parent")?;
    let store = ArtifactStore::open(&store_dir.join("artifacts"))?;
    Ok(verify_results(path, &store)?)
}

fn base_row() -> Result<Value> {
    let mut row: Value = serde_json::from_slice(NATIVE_ACCEPTED)?;
    row["report"]["resolution"]["method"] = json!("deterministic");
    row["report"]["job"]["rankings"] = Value::Null;
    row["report"]["revision"] = json!(ROW_PROTOCOL_REVISION);
    Ok(row)
}

fn accepted_row(method: &str) -> Result<Value> {
    let mut row = base_row()?;
    row["report"]["resolution"]["method"] = json!(method);
    row["_fixture_recompute_assessment"] = json!(true);
    Ok(row)
}

fn contradictory_row() -> Result<Value> {
    let mut row: Value = serde_json::from_slice(NATIVE_CONTRADICTION)?;
    row["report"]["resolution"] = json!({"status":"review_required"});
    row["report"]["job"]["rankings"] = Value::Null;
    row["report"]["revision"] = json!(ROW_PROTOCOL_REVISION);
    row["profile_artifacts"][0]["complete"] = json!(false);
    row["_fixture_recompute_assessment"] = json!(true);
    Ok(row)
}

fn source_snapshot(store: &ArtifactStore) -> Result<String> {
    let snapshot = SourceSnapshot {
        revision: athletic_rust_pipeline::runtime::acquisition::ACQUISITION_REVISION.to_owned(),
        source_origin: ORIGIN.to_owned(),
        label: "synthetic-result-verifier".to_owned(),
        rankings: None,
    };
    let bytes = serde_json::to_vec(&snapshot)?;
    Ok(store.put_bytes(&bytes)?.as_str().to_owned())
}

fn raw_digest_map(store: &ArtifactStore, relay: bool) -> Result<BTreeMap<String, String>> {
    let mut relay_tf = serde_json::from_slice::<Value>(RAW_TF)?;
    if relay {
        relay_tf["relayTeamMembers"] = json!([{"TeamID":99,"AthleteID":1001}]);
        relay_tf["eventsTF"][0]["PersonalEvent"] = json!(false);
        relay_tf["resultsTF"][0]["AthleteID"] = json!(99);
    }
    let relay_tf = serde_json::to_vec(&relay_tf)?;
    let tf_bytes: &[u8] = if relay { relay_tf.as_slice() } else { RAW_TF };
    let team = br#"{"team":{"ID":501,"Name":"Central","State":"TX","Level":4}}"#;
    let tf_digest = store.put_bytes(tf_bytes)?.as_str().to_owned();
    let xc_digest = store.put_bytes(RAW_XC)?.as_str().to_owned();
    let html_digest = store.put_bytes(RAW_HTML)?.as_str().to_owned();
    let team_digest = store.put_bytes(team)?.as_str().to_owned();
    Ok(BTreeMap::from([
        (OLD_TF.to_owned(), tf_digest),
        (OLD_XC.to_owned(), xc_digest),
        (OLD_HTML.to_owned(), html_digest),
        (OLD_TEAM.to_owned(), team_digest),
    ]))
}

fn remap(value: &mut Value, documents: &BTreeMap<String, String>) {
    match value {
        Value::String(text) => {
            if let Some(replacement) = documents.get(text) {
                *text = replacement.clone();
            } else if text.contains("127.0.0.1:18087") {
                *text = text.replace("127.0.0.1:18087", "127.0.0.1:18081");
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|item| remap(item, documents)),
        Value::Object(items) => items.values_mut().for_each(|item| remap(item, documents)),
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}
fn remap_candidate_id(value: &mut Value) {
    match value {
        Value::Number(number) if number.as_u64() == Some(1001) => *value = json!(1008),
        Value::String(text) => {
            *text = text
                .replace("/1001/", "/1008/")
                .replace("athleteId=1001", "athleteId=1008")
                .replace("tf-1001", "tf-1008")
                .replace("xc-1001", "xc-1008")
                .replace("\"id\":1001,", "\"id\":1008,");
        }
        Value::Array(items) => items.iter_mut().for_each(remap_candidate_id),
        Value::Object(items) => items.values_mut().for_each(remap_candidate_id),
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}
fn freshen_candidate_operations(value: &mut Value, label: &str) {
    if let Some(operations) = value["operations"].as_array_mut() {
        operations
            .iter_mut()
            .enumerate()
            .for_each(|(index, operation)| {
                let digest = format!(
                    "{:x}",
                    Sha256::digest(format!("{label}-1008-{index}").as_bytes())
                );
                operation["operation"] = json!(digest);
            });
    }
}
fn candidate_documents(store: &ArtifactStore) -> Result<BTreeMap<String, String>> {
    let mut tf = serde_json::from_slice::<Value>(RAW_TF)?;
    let mut xc = serde_json::from_slice::<Value>(RAW_XC)?;
    remap_candidate_id(&mut tf);
    remap_candidate_id(&mut xc);
    let html = String::from_utf8_lossy(RAW_HTML).replace("1001", "1008");
    let values = [
        (OLD_TF, serde_json::to_vec(&tf)?),
        (OLD_XC, serde_json::to_vec(&xc)?),
        (OLD_HTML, html.into_bytes()),
    ];
    values
        .into_iter()
        .try_fold(BTreeMap::new(), |mut map, (old, bytes)| {
            let digest = store.put_bytes(&bytes)?.as_str().to_owned();
            map.insert(old.to_owned(), digest);
            Ok(map)
        })
}

fn operation_attempt(
    store: &ArtifactStore,
    operation: &str,
    response: &Value,
    body: Option<Value>,
) -> Result<String> {
    let operation = EvidenceDigest::parse(operation)?;
    let source_url = response["source_url"]
        .as_str()
        .context("captured response URL")?;
    let captured = json!({
        "request": {
            "url": source_url,
            "semantic_url": source_url,
            "action": {"action": "fetch", "body": body}
        },
        "receipt": response,
        "code": null,
        "status": response["http_status"],
        "message": "",
        "retryable": false,
        "retry_after_ms": 0
    });
    Ok(store
        .record_attempt(&operation, &captured)?
        .as_str()
        .to_owned())
}
fn refresh_response_lengths(row: &mut Value, store: &ArtifactStore) -> Result<()> {
    ["profile_artifacts", "identity_artifacts"]
        .into_iter()
        .try_for_each(|field| {
            row[field]
                .as_array_mut()
                .context(field)?
                .iter_mut()
                .try_for_each(|artifact| {
                    artifact["responses"]
                        .as_array_mut()
                        .context("responses")?
                        .iter_mut()
                        .try_for_each(|response| {
                            let digest = EvidenceDigest::parse(
                                response["digest"].as_str().context("response digest")?,
                            )?;
                            response["bytes"] = json!(store.get_bytes(&digest)?.len());
                            response["rankings"] = Value::Null;
                            Ok::<(), anyhow::Error>(())
                        })
                })
        })
}

fn materialize_operations(
    store: &ArtifactStore,
    operations: &mut [Value],
    responses: &[Value],
    known: &mut BTreeMap<String, String>,
) -> Result<()> {
    operations
        .iter_mut()
        .zip(responses)
        .try_for_each(|(operation, response)| {
            let operation_id = operation["operation"]
                .as_str()
                .context("operation digest")?;
            let attempt = match known.get(operation_id) {
                Some(digest) => digest.clone(),
                None => {
                    let digest = operation_attempt(store, operation_id, response, None)?;
                    known.insert(operation_id.to_owned(), digest.clone());
                    digest
                }
            };
            operation["attempts"] = json!([attempt]);
            operation["observed_attempts"] = json!(1);
            operation["maximum_retries"] = json!(3);
            operation["ownership"] = json!("workflow_controlled");
            Ok::<(), anyhow::Error>(())
        })
}

fn search_evidence(
    store: &ArtifactStore,
    source: &athletic_rust_pipeline::model::SourceRecord,
    snapshot: &str,
    known: &mut BTreeMap<String, String>,
    two_candidates: bool,
) -> Result<Vec<String>> {
    query_plan(source)?.into_iter().enumerate().map(|(index, query)| {
        let suffix = match query.sport() {
            athletic_rust_pipeline::domain::evidence::Sport::TrackField => "track-and-field",
            athletic_rust_pipeline::domain::evidence::Sport::CrossCountry => "cross-country",
        };
        let results = if two_candidates {
            format!("<tr><td><a href=\"/athlete/1001/{suffix}/all\">Ada Runner</a></td></tr><tr><td><a href=\"/athlete/1008/{suffix}/all\">Ada Runner</a></td></tr>")
        } else {
            format!("<tr><td><a href=\"/athlete/1001/{suffix}/all\">Ada Runner</a></td></tr>")
        };
        let count = if two_candidates { 2 } else { 1 };
        let body = json!({"d":{"count":count,"pager":"","results":results}});
        let bytes = serde_json::to_vec(&body)?;
        let receipt_digest = store.put_bytes(&bytes)?.as_str().to_owned();
        let response = json!({
            "digest": receipt_digest,
            "source_url": format!("{ORIGIN}Search.aspx/runSearch"),
            "http_status": 200,
            "media_type": "application/json",
            "bytes": bytes.len(),
            "fetched_at_unix_ms": 1,
            "elapsed_ms": 1,
            "rankings": null
        });
        let operation = format!("{:x}", Sha256::digest(format!("query-{snapshot}-{index}").as_bytes()));
        let body = json!({"q": query.text(), "fq": format!("t:a {}", query.filter()), "start": 0});
        let attempt = match known.get(&operation) {
            Some(digest) => digest.clone(),
            None => {
                let digest = operation_attempt(store, &operation, &response, Some(body))?;
                known.insert(operation.clone(), digest.clone());
                digest
            }
        };
        let request = QueryPage {
            response: serde_json::from_value(response.clone())?,
            parsed: store.put_bytes(&serde_json::to_vec(&parse_page(
                &query,
                0,
                EvidenceDigest::parse(&receipt_digest)?,
                &bytes,
            )?)?)?,
            retries: serde_json::from_value(json!({
                "ownership":"workflow_controlled","operation":operation,"maximum_retries":3,
                "observed_attempts":1,"attempts":[attempt]
            }))?,
            previous_responses: Vec::new(),
        };
        let evidence = QueryEvidence {
            query,
            pages: vec![request],
            complete: true,
            issues: Vec::new(),
            failures: Vec::new(),
        };
        Ok(store.put_bytes(&serde_json::to_vec(&evidence)?)?.as_str().to_owned())
    }).collect()
}

fn materialize_row(
    mut row: Value,
    store: &ArtifactStore,
    known: &mut BTreeMap<String, String>,
) -> Result<Value> {
    let recompute_assessment = row
        .get("_fixture_recompute_assessment")
        .and_then(Value::as_bool)
        .is_some_and(|value| value);
    let relay = row
        .get("_fixture_relay")
        .and_then(Value::as_bool)
        .is_some_and(|value| value);
    let two_candidates = row
        .get("_fixture_two_candidates")
        .and_then(Value::as_bool)
        .is_some_and(|value| value);
    if let Some(object) = row.as_object_mut() {
        object.remove("_fixture_recompute_assessment");
        object.remove("_fixture_relay");
        object.remove("_fixture_two_candidates");
    }
    if !recompute_assessment {
        if let Some(value) = row.get("assessment").filter(|value| !value.is_null()) {
            let typed: AssessmentFixture = serde_json::from_value(value.clone())?;
            let digest = store.put_bytes(&serde_json::to_vec(&typed)?)?;
            row["report"]["assessment"] = json!(digest.as_str());
        }
        let profiles = row["profile_artifacts"]
            .as_array()
            .context("profile artifacts")?
            .clone();
        let probes = row["identity_artifacts"]
            .as_array()
            .context("identity artifacts")?
            .clone();
        let profile_digests = profiles
            .iter()
            .map(|value| {
                let typed: ProfileAcquisition = serde_json::from_value(value.clone())?;
                Ok::<_, anyhow::Error>(
                    store
                        .put_bytes(&serde_json::to_vec(&typed)?)?
                        .as_str()
                        .to_owned(),
                )
            })
            .collect::<Result<Vec<_>>>()?;
        let probe_digests = probes
            .iter()
            .map(|value| {
                let typed: ProfileProbe = serde_json::from_value(value.clone())?;
                Ok::<_, anyhow::Error>(
                    store
                        .put_bytes(&serde_json::to_vec(&typed)?)?
                        .as_str()
                        .to_owned(),
                )
            })
            .collect::<Result<Vec<_>>>()?;
        if let Some(candidates) = row["report"]["candidates"].as_array_mut() {
            candidates
                .iter_mut()
                .enumerate()
                .for_each(|(index, candidate)| {
                    let Some(kind) = candidate.as_object_mut() else {
                        return;
                    };
                    let Some(object) = kind.values_mut().next().and_then(Value::as_object_mut)
                    else {
                        return;
                    };
                    if let Some(value) = probe_digests.get(index) {
                        object.insert("probe".to_owned(), json!(value));
                    }
                    if let Some(value) = profile_digests.get(index) {
                        object.insert("profile".to_owned(), json!(value));
                    }
                });
        }
        let report: RowReport = serde_json::from_value(row["report"].clone())?;
        let digest = store.put_bytes(&serde_json::to_vec(&report)?)?;
        row["report_digest"] = json!(digest.as_str());
        return Ok(row);
    }
    if row["report"]["discovery"].is_null() && row["assessment"].is_null() {
        row["identity_artifacts"] = json!([]);
        row["profile_artifacts"] = json!([]);
        row["performance_evidence"] = json!([]);
        row["report"]["candidates"] = json!([]);
        row["report"]["query_evidence"] = json!([]);
        row["report"]["assessment"] = Value::Null;
        let report: RowReport = serde_json::from_value(row["report"].clone())?;
        row["report_digest"] = json!(typed_digest(&report)?);
        return Ok(row);
    }
    if two_candidates {
        let documents = candidate_documents(store)?;
        let mut profile = row["profile_artifacts"][0].clone();
        let mut probe = row["identity_artifacts"][0].clone();
        remap_candidate_id(&mut profile);
        remap_candidate_id(&mut probe);
        remap(&mut profile, &documents);
        remap(&mut probe, &documents);
        freshen_candidate_operations(&mut profile, "candidate");
        freshen_candidate_operations(&mut probe, "candidate");
        row["profile_artifacts"]
            .as_array_mut()
            .context("profile artifacts")?
            .push(profile);
        row["identity_artifacts"]
            .as_array_mut()
            .context("identity artifacts")?
            .push(probe);
    }
    let documents = raw_digest_map(store, relay)?;
    remap(&mut row, &documents);
    if relay {
        row["profile_artifacts"][0]["profile"]["results"][0]["attribution"] =
            json!({"kind":"verified_relay_member","relay_athlete_id":99});
        row["profile_artifacts"][0]["profile"]["results"][0]["result_url"] = Value::Null;
        row["identity_artifacts"][0]["profiles"][0]["results"][0] =
            row["profile_artifacts"][0]["profile"]["results"][0].clone();
    }
    row["performance_evidence"] = row["profile_artifacts"]
        .as_array()
        .context("profile artifacts")?
        .iter()
        .filter_map(|artifact| artifact["profile"]["results"].as_array())
        .flatten()
        .cloned()
        .collect::<Vec<_>>()
        .into();
    refresh_response_lengths(&mut row, store)?;
    let snapshot = source_snapshot(store)?;
    row["report"]["job"]["snapshot"] = json!(snapshot);
    row["profile_artifacts"]
        .as_array_mut()
        .context("profile artifacts")?
        .iter_mut()
        .try_for_each(|value| {
            let responses = value["responses"]
                .as_array()
                .context("profile responses")?
                .clone();
            let operations = value["operations"]
                .as_array_mut()
                .context("profile operations")?;
            materialize_operations(store, operations, &responses, known)
        })?;
    row["identity_artifacts"]
        .as_array_mut()
        .context("identity artifacts")?
        .iter_mut()
        .try_for_each(|value| {
            let responses = value["responses"]
                .as_array()
                .context("probe responses")?
                .clone();
            let operations = value["operations"]
                .as_array_mut()
                .context("probe operations")?;
            materialize_operations(store, operations, &responses, known)
        })?;
    let source: athletic_rust_pipeline::model::SourceRecord =
        serde_json::from_value(row["source"].clone())?;
    let candidate_ids = row["profile_artifacts"]
        .as_array()
        .context("profile artifacts")?
        .iter()
        .filter_map(|value| value["athlete_id"].as_u64())
        .collect::<Vec<_>>();
    let query_artifacts = search_evidence(store, &source, &snapshot, known, two_candidates)?;
    row["discovery"] = json!({
        "job": row["report"]["job"], "candidate_ids": candidate_ids,
        "query_artifacts": query_artifacts, "complete": true, "issues": [],
        "rankings": null
    });
    let discovery: DiscoverySummary = serde_json::from_value(row["discovery"].clone())?;
    let discovery_digest = store
        .put_bytes(&serde_json::to_vec(&discovery)?)?
        .as_str()
        .to_owned();
    row["report"]["discovery"] = json!(discovery_digest);
    row["report"]["query_evidence"] = json!(discovery.query_artifacts);
    let profiles = row["profile_artifacts"]
        .as_array()
        .context("profile artifacts")?
        .clone();
    let probes = row["identity_artifacts"]
        .as_array()
        .context("identity artifacts")?
        .clone();
    let typed_profiles = profiles
        .iter()
        .map(|value| serde_json::from_value::<ProfileAcquisition>(value.clone()))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let typed_probes = probes
        .iter()
        .map(|value| serde_json::from_value::<ProfileProbe>(value.clone()))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let profile_digests = typed_profiles
        .iter()
        .map(typed_digest)
        .collect::<Result<Vec<_>>>()?;
    let probe_digests = typed_probes
        .iter()
        .map(typed_digest)
        .collect::<Result<Vec<_>>>()?;
    row["report"]["candidates"] = typed_profiles
        .iter()
        .zip(typed_probes.iter())
        .enumerate()
        .map(|(index, (profile, probe))| {
            let payload = json!({
                "athlete_id": profile.athlete_id,
                "probe": probe_digests.get(index),
                "profile": profile_digests.get(index)
            });
            if profile.complete
                && profile.failures.is_empty()
                && profile.profile.is_some()
                && probe.complete
                && probe.failures.is_empty()
            {
                json!({"Complete": payload})
            } else {
                json!({"Incomplete": {
                    "athlete_id": profile.athlete_id,
                    "probe": probe_digests.get(index),
                    "profile": profile_digests.get(index),
                    "issues": ["retained candidate acquisition is incomplete"]
                }})
            }
        })
        .collect::<Vec<_>>()
        .into();
    if recompute_assessment && row["assessment"].is_object() {
        let evidence = typed_profiles.iter().filter_map(|value| {
            value.profile.as_ref().map(|profile| {
                if value.complete && value.failures.is_empty() {
                    CandidateEvidence::Complete(profile)
                } else {
                    CandidateEvidence::Incomplete {
                        athlete_id: value.athlete_id,
                        profile: Some(profile),
                    }
                }
            })
        });
        let complete = typed_profiles
            .iter()
            .all(|value| value.complete && value.failures.is_empty() && value.profile.is_some());
        let search = if complete {
            SearchCompleteness::Complete {
                evidence: EvidenceDigest::parse(&discovery_digest)?,
            }
        } else {
            SearchCompleteness::Incomplete {
                reasons: vec!["retained profile acquisition is incomplete".to_owned()],
            }
        };
        let assessment = decision::assess(&source, evidence, search)?;
        row["assessment"] = serde_json::to_value(&assessment)?;
        let assessment_digest = store
            .put_bytes(&serde_json::to_vec(&assessment)?)?
            .as_str()
            .to_owned();
        row["report"]["assessment"] = json!(assessment_digest);
    } else if row["assessment"].is_object() {
        let assessment: AssessmentFixture = serde_json::from_value(row["assessment"].clone())?;
        let assessment_digest = store.put_bytes(&serde_json::to_vec(&assessment)?)?;
        row["report"]["assessment"] = json!(assessment_digest.as_str());
    } else if row["assessment"].is_null() {
        row["report"]["assessment"] = Value::Null;
    }
    let report: RowReport = serde_json::from_value(row["report"].clone())?;
    row["report_digest"] = json!(typed_digest(&report)?);
    Ok(row)
}

fn put_typed<T: Serialize>(
    store: &ArtifactStore,
    value: &T,
    expected: Option<&str>,
    label: &str,
) -> Result<()> {
    let actual = store.put_bytes(&serde_json::to_vec(value)?)?;
    if expected.is_some_and(|digest| digest != actual.as_str()) {
        anyhow::bail!("{label} fixture digest differs from stored bytes");
    }
    Ok(())
}

fn seed_artifacts(row: &Value, store: &ArtifactStore) -> Result<()> {
    if let Some(value) = row.get("discovery").filter(|value| !value.is_null()) {
        let typed: DiscoverySummary = serde_json::from_value(value.clone())?;
        put_typed(
            store,
            &typed,
            row["report"]["discovery"].as_str(),
            "discovery",
        )?;
    }
    if let Some(value) = row.get("assessment").filter(|value| !value.is_null()) {
        let typed: AssessmentFixture = serde_json::from_value(value.clone())?;
        put_typed(
            store,
            &typed,
            row["report"]["assessment"].as_str(),
            "assessment",
        )?;
    }
    row["identity_artifacts"]
        .as_array()
        .context("identity artifacts")?
        .iter()
        .try_for_each(|value| {
            let typed: ProfileProbe = serde_json::from_value(value.clone())?;
            put_typed(store, &typed, None, "probe")
        })?;
    row["profile_artifacts"]
        .as_array()
        .context("profile artifacts")?
        .iter()
        .try_for_each(|value| {
            let typed: ProfileAcquisition = serde_json::from_value(value.clone())?;
            put_typed(store, &typed, None, "profile")
        })?;
    if let Some(value) = row.get("report").filter(|value| !value.is_null()) {
        let typed: RowReport = serde_json::from_value(value.clone())?;
        put_typed(store, &typed, row["report_digest"].as_str(), "report")?;
    }
    Ok(())
}

fn seed_store(path: &Path, rows: &[Value]) -> Result<Vec<Value>> {
    let root = path
        .parent()
        .context("fixture path has no parent")?
        .join("artifacts");
    let store = ArtifactStore::open(&root)?;
    let mut known = BTreeMap::new();
    rows.iter()
        .cloned()
        .try_fold(Vec::new(), |mut seeded, row| {
            let row = materialize_row(row, &store, &mut known)?;
            seed_artifacts(&row, &store)?;
            seeded.push(row);
            Ok(seeded)
        })
}

fn write(path: &Path, rows: &[Value]) -> TestResult {
    let rows = seed_store(path, rows)?;
    let body = rows
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()?
        .join("\n");
    fs::write(path, format!("{body}\n"))?;
    Ok(())
}

fn write_value(path: &Path, value: Value) -> TestResult {
    fs::write(path, format!("{}\n", serde_json::to_string(&value)?))?;
    Ok(())
}

fn local_selection(mut row: Value) -> Value {
    row["report"]["resolution"] =
        json!({"status":"accepted","athlete_id":1001,"method":"local_review"});
    let evidence = row["profile_artifacts"][0]["profile"]["results"][0]["evidence"].clone();
    let request = row["profile_artifacts"][0]["responses"][0]["digest"].clone();
    let response = row["profile_artifacts"][0]["responses"][0].clone();
    row["report"]["review"] = json!({
        "outcome":"reviewed","lane":"q5_5090","verdict":{"decision":"select","athlete_id":1001,
        "reason":"Selected supplied school evidence","evidence":[evidence]},"request":request,
        "response":response,"previous_responses":[],"retries":{"ownership":"not_attempted"}
    });
    row
}

#[test]
fn verifies_deterministic_positive_from_retained_evidence() -> TestResult {
    let directory = tempdir()?;
    let path = directory.path().join("detail.jsonl");
    write(&path, &[accepted_row("deterministic")?])?;
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
    let trusted = seed_store(&forged_path, &[accepted_row("deterministic")?])?
        .into_iter()
        .next()
        .context("trusted row")?;
    let mut forged = trusted;
    forged["report"]["resolution"]["athlete_id"] = json!(99);
    write_value(&forged_path, forged)?;
    assert!(verify(&forged_path).is_err());
    let duplicate_path = directory.path().join("duplicate.jsonl");
    write(
        &duplicate_path,
        &[
            accepted_row("deterministic")?,
            accepted_row("deterministic")?,
        ],
    )?;
    assert!(verify(&duplicate_path).is_err());
    Ok(())
}

#[test]
fn rejects_local_acceptance_without_review_artifact() -> TestResult {
    let directory = tempdir()?;
    let path = directory.path().join("local.jsonl");
    let trusted = seed_store(&path, &[accepted_row("local_review")?])?
        .into_iter()
        .next()
        .context("trusted row")?;
    let mut row = trusted;
    row["assessment"] = Value::Null;
    write_value(&path, row)?;
    assert!(verify(&path).is_err());
    Ok(())
}

#[test]
fn preserves_source_validation_review_without_assessment() -> TestResult {
    let directory = tempdir()?;
    let path = directory.path().join("review.jsonl");
    let mut row = accepted_row("deterministic")?;
    row["source"]["fields"]["Person First"] = json!("");
    row["discovery"] = Value::Null;
    row["report"]["discovery"] = Value::Null;
    row["report"]["candidates"] = json!([]);
    row["report"]["query_evidence"] = json!([]);
    row["report"]["review"] = Value::Null;
    row["report"]["issues"] = json!(["source identity is incomplete"]);
    row["report"]["resolution"] = json!({"status":"review_required"});
    row["report"]["assessment"] = Value::Null;
    row["assessment"] = Value::Null;
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
    let row = contradictory_row()?;
    write(&path, &[row])?;
    let report = verify(&path)?;
    assert_eq!(report.review_rows, 1);
    assert_eq!(report.accepted_rows, 0);
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
        let path = directory.path().join(format!("stale-{index}.jsonl"));
        let trusted = seed_store(&path, &[accepted_row("deterministic")?])?
            .into_iter()
            .next()
            .context("trusted row")?;
        let mut row = trusted;
        *row.pointer_mut(pointer).context("fixture artifact field")? = replacement;
        write_value(&path, row)?;
        assert!(
            verify(&path).is_err(),
            "accepted stale digest for {pointer}"
        );
    }
    Ok(())
}

#[test]
fn rejects_rehashed_canonical_assessment_flag_tampering() -> TestResult {
    let directory = tempdir()?;
    let path = directory.path().join("assessment-flag-forgery.jsonl");
    let trusted = seed_store(&path, &[accepted_row("deterministic")?])?
        .into_iter()
        .next()
        .context("trusted row")?;
    let mut row = trusted;
    row["assessment"]["candidates"][0]["hard_eligible"] = json!(false);
    write(&path, &[row])?;
    assert!(verify(&path).is_err());
    Ok(())
}

#[test]
fn rejects_performance_projection_that_differs_from_retained_results() -> TestResult {
    let directory = tempdir()?;
    let path = directory.path().join("performance-forgery.jsonl");
    let trusted = seed_store(&path, &[accepted_row("deterministic")?])?
        .into_iter()
        .next()
        .context("trusted row")?;
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
    let trusted = seed_store(&path, &[accepted_row("deterministic")?])?
        .into_iter()
        .next()
        .context("trusted row")?;
    let encoded = serde_json::to_string(&trusted)?;
    let remainder = encoded.strip_prefix('{').context("fixture JSON object")?;
    fs::write(&path, format!("{{\"report\":null,{remainder}\n"))?;
    assert!(verify(&path).is_err());
    Ok(())
}

#[test]
fn rejects_indistinguishable_local_selection_with_rehashed_evidence() -> TestResult {
    let directory = tempdir()?;
    let path = directory.path().join("indistinguishable.jsonl");
    let mut row = accepted_row("deterministic")?;
    row["_fixture_two_candidates"] = json!(true);
    row["report"]["resolution"] = json!({"status":"review_required"});
    let trusted = seed_store(&path, &[row])?
        .into_iter()
        .next()
        .context("trusted row")?;
    write_value(&path, trusted.clone())?;
    assert_eq!(verify(&path)?.review_rows, 1);
    let mut row = trusted;
    row["report"]["resolution"]["method"] = json!("local_review");
    row["assessment"]["decision"] = json!("deterministic_accepted");
    row["assessment"]["verified"] = json!({"athlete_id":1001});
    write(&path, &[local_selection(row)])?;
    assert!(verify(&path).is_err());
    Ok(())
}

#[test]
fn rejects_local_review_claim_for_deterministic_assessment() -> TestResult {
    let directory = tempdir()?;
    let path = directory.path().join("wrong-review-state.jsonl");
    let trusted = seed_store(&path, &[accepted_row("deterministic")?])?
        .into_iter()
        .next()
        .context("trusted row")?;
    write_value(&path, local_selection(trusted))?;
    assert!(verify(&path).is_err());
    Ok(())
}

#[test]
fn accepts_reused_query_with_case_and_stage_variant() -> TestResult {
    let directory = tempdir()?;
    let path = directory.path().join("reused-query.jsonl");
    let mut trusted = seed_store(&path, &[accepted_row("deterministic")?])?
        .into_iter()
        .next()
        .context("trusted row")?;
    let store = ArtifactStore::open(&directory.path().join("artifacts"))?;
    let artifact = trusted["discovery"]["query_artifacts"][0]
        .as_str()
        .context("query artifact digest")?;
    let digest = EvidenceDigest::parse(artifact)?;
    let mut evidence: QueryEvidence = serde_json::from_slice(&store.get_bytes(&digest)?)?;
    let mut query = serde_json::to_value(&evidence.query)?;
    query["query"] = json!(evidence.query.text().to_ascii_uppercase());
    query["stage"] = json!(99);
    evidence.query = serde_json::from_value(query)?;
    let page = evidence.pages.first_mut().context("query page")?;
    let mut parsed: SearchPage = serde_json::from_slice(&store.get_bytes(&page.parsed)?)?;
    parsed.query = evidence.query.clone();
    page.parsed = store.put_bytes(&serde_json::to_vec(&parsed)?)?;
    let response = serde_json::to_value(&page.response)?;
    let body = json!({
        "q": evidence.query.text(),
        "fq": format!("t:a {}", evidence.query.filter()),
        "start": 0
    });
    let operation = format!(
        "{:x}",
        Sha256::digest(format!("reused-query-{}", evidence.query.cache_identity()).as_bytes())
    );
    let attempt = operation_attempt(&store, &operation, &response, Some(body))?;
    let mut retries = serde_json::to_value(&page.retries)?;
    retries["operation"] = json!(operation);
    retries["attempts"] = json!([attempt]);
    page.retries = serde_json::from_value(retries)?;
    let replacement = store.put_bytes(&serde_json::to_vec(&evidence)?)?;
    trusted["discovery"]["query_artifacts"][0] = json!(replacement.as_str());
    let discovery: DiscoverySummary = serde_json::from_value(trusted["discovery"].clone())?;
    let discovery_digest = store.put_bytes(&serde_json::to_vec(&discovery)?)?;
    trusted["report"]["discovery"] = json!(discovery_digest.as_str());
    trusted["report"]["query_evidence"] = json!(discovery.query_artifacts);
    let mut assessment: AssessmentFixture = serde_json::from_value(trusted["assessment"].clone())?;
    assessment.search = SearchCompleteness::Complete {
        evidence: discovery_digest,
    };
    trusted["assessment"] = serde_json::to_value(&assessment)?;
    let assessment_digest = store.put_bytes(&serde_json::to_vec(&assessment)?)?;
    trusted["report"]["assessment"] = json!(assessment_digest.as_str());
    let report: RowReport = serde_json::from_value(trusted["report"].clone())?;
    let report_digest = store.put_bytes(&serde_json::to_vec(&report)?)?;
    trusted["report_digest"] = json!(report_digest.as_str());
    drop(store);
    write_value(&path, trusted)?;
    assert_eq!(verify(&path)?.accepted_rows, 1);
    Ok(())
}

#[test]
fn verifies_relay_identity_distinct_from_confirmed_member() -> TestResult {
    let directory = tempdir()?;
    let path = directory.path().join("relay-member.jsonl");
    let mut row = accepted_row("deterministic")?;
    row["_fixture_relay"] = json!(true);
    write(&path, &[row])?;
    assert_eq!(verify(&path)?.accepted_rows, 1);
    Ok(())
}

#[test]
fn rejects_profile_identity_swap_with_matching_inner_url() -> TestResult {
    let directory = tempdir()?;
    let path = directory.path().join("swapped-profile.jsonl");
    let trusted = seed_store(&path, &[accepted_row("deterministic")?])?
        .into_iter()
        .next()
        .context("trusted row")?;
    let mut row = trusted;
    row["profile_artifacts"][0]["profile"]["athlete_id"] = json!(8);
    row["profile_artifacts"][0]["profile"]["profile_url"] =
        json!("https://www.athletic.net/athlete/8/track-and-field");
    write_value(&path, row)?;
    assert!(verify(&path).is_err());
    Ok(())
}
