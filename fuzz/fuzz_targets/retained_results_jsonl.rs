#![no_main]

#[path = "retained_results_jsonl/assessment.rs"]
mod assessment;
#[path = "retained_results_jsonl/references.rs"]
mod references;

use anyhow::{bail, Context, Result};
use athletic_rust_pipeline::{
    domain::{
        decision::{CandidateReason, Decision, SearchCompleteness},
        identity::{AthleteId, EvidenceDigest},
    },
    result_verify::{verify_results, ResultVerificationReport},
    runtime::{
        acquisition::{ProfileAcquisition, ProfileProbe, QueryEvidence, QueryPage},
        identity::fingerprint,
        row_protocol::{DiscoverySummary, RowReport},
        run_protocol::SourceSnapshot,
    },
    search::{parse_page, query_plan},
    store::ArtifactStore,
};
use libfuzzer_sys::fuzz_target;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    io::Write,
    sync::{Once, OnceLock},
};
use tempfile::{tempdir, NamedTempFile, TempDir};

const MAX_INPUT_BYTES: usize = 2 * 1024 * 1024;
const TRUSTED_WITNESSES: [&[u8]; 3] = [
    include_bytes!("../fixtures/retained_results_jsonl/seed-valid-accepted.json"),
    include_bytes!("../fixtures/retained_results_jsonl/seed-valid-no-match.json"),
    include_bytes!("../fixtures/retained_results_jsonl/seed-valid-excluded.json"),
];
static WITNESSES: Once = Once::new();
static FIXTURE_STORE: OnceLock<std::result::Result<FixtureStore, String>> = OnceLock::new();

struct FixtureStore {
    _root: TempDir,
    store: ArtifactStore,
    witnesses: Vec<Vec<u8>>,
    references: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct AssessmentWire {
    decision: Decision,
    candidates: Vec<CandidateReason>,
    search: SearchCompleteness,
    #[serde(default)]
    verified: Option<VerifiedWire>,
}

#[derive(Debug, Deserialize, Serialize)]
struct VerifiedWire {
    athlete_id: AthleteId,
}

fn trusted_store() -> Result<&'static FixtureStore> {
    match FIXTURE_STORE.get_or_init(|| build_fixture_store().map_err(|error| format!("{error:#}")))
    {
        Ok(fixtures) => Ok(fixtures),
        Err(error) => bail!("trusted retained-results fixtures are invalid: {error}"),
    }
}
fn build_fixture_store() -> Result<FixtureStore> {
    let root = tempdir().context("creating retained-results fixture directory")?;
    let store = ArtifactStore::open(&root.path().join("artifacts"))
        .context("opening retained-results fixture store")?;
    seed_raw_documents(&store)?;
    let mut references = BTreeMap::new();
    let witnesses = TRUSTED_WITNESSES
        .iter()
        .map(|bytes| {
            let mut row: Value =
                serde_json::from_slice(bytes).context("decoding trusted witness")?;
            let original = row.clone();
            hydrate_witness(&mut row, &store)?;
            references::collect(&original, &row, &mut references)?;
            seed_witness(&store, &row)?;
            let mut encoded = serde_json::to_vec(&row)?;
            encoded.push(b'\n');
            Ok::<Vec<u8>, anyhow::Error>(encoded)
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(FixtureStore {
        _root: root,
        store,
        witnesses,
        references,
    })
}

fn seed_witness(store: &ArtifactStore, row: &Value) -> Result<()> {
    let Some(report_value) = row.get("report").filter(|value| !value.is_null()) else {
        return Ok(());
    };
    let report: RowReport =
        serde_json::from_value(report_value.clone()).context("decoding trusted row report")?;
    let report_digest = digest_field(row, "report_digest")?;
    seed_typed::<RowReport>(store, report_value, &report_digest, "row report")?;
    if let Some(discovery_digest) = report.discovery.as_ref() {
        let discovery_value = row
            .get("discovery")
            .filter(|value| !value.is_null())
            .context("trusted report has no discovery artifact")?;
        seed_typed::<DiscoverySummary>(
            store,
            discovery_value,
            discovery_digest,
            "discovery summary",
        )?;
    }
    if let Some(assessment_digest) = report.assessment.as_ref() {
        let assessment_value = row
            .get("assessment")
            .filter(|value| !value.is_null())
            .context("trusted report has no assessment artifact")?;
        seed_typed::<AssessmentWire>(store, assessment_value, assessment_digest, "assessment")?;
    }
    row["identity_artifacts"]
        .as_array()
        .context("trusted identity artifacts are not an array")?
        .iter()
        .try_for_each(|value| {
            let typed: ProfileProbe = serde_json::from_value(value.clone())?;
            store.put_bytes(&serde_json::to_vec(&typed)?).map(|_| ())
        })?;
    row["profile_artifacts"]
        .as_array()
        .context("trusted profile artifacts are not an array")?
        .iter()
        .try_for_each(|value| {
            let typed: ProfileAcquisition = serde_json::from_value(value.clone())?;
            store.put_bytes(&serde_json::to_vec(&typed)?).map(|_| ())
        })?;
    Ok(())
}

fn digest_field(row: &Value, field: &str) -> Result<EvidenceDigest> {
    let raw = row
        .get(field)
        .and_then(Value::as_str)
        .with_context(|| format!("trusted witness has no {field}"))?;
    EvidenceDigest::parse(raw).with_context(|| format!("invalid trusted {field}"))
}

fn seed_typed<T>(
    store: &ArtifactStore,
    raw: &Value,
    expected: &EvidenceDigest,
    label: &str,
) -> Result<()>
where
    T: for<'de> Deserialize<'de> + Serialize,
{
    let typed: T =
        serde_json::from_value(raw.clone()).with_context(|| format!("decoding trusted {label}"))?;
    let bytes =
        serde_json::to_vec(&typed).with_context(|| format!("serializing trusted {label}"))?;
    let actual = store
        .put_bytes(&bytes)
        .with_context(|| format!("storing trusted {label}"))?;
    if actual != *expected {
        bail!("trusted {label} digest differs from native witness reference");
    }
    Ok(())
}

fn remap_team_digest(value: &mut Value, replacement: &str) {
    match value {
        Value::String(text) if text == OLD_TEAM_DIGEST => {
            *text = replacement.to_string();
        }
        Value::Array(items) => {
            for item in items.iter_mut() {
                remap_team_digest(item, replacement);
            }
        }
        Value::Object(items) => {
            for item in items.values_mut() {
                remap_team_digest(item, replacement);
            }
        }
        _ => {}
    }
}

const OLD_TEAM_DIGEST: &str = "a5d54ee54f8f147ff02fea9aafd3c7836e57c77e4cce12cb191836aa499a1135";

fn captured_attempt(
    store: &ArtifactStore,
    operation: &str,
    response: &Value,
    body: Option<Value>,
) -> Result<String> {
    let operation = EvidenceDigest::parse(operation)?;
    let source_url = response["source_url"]
        .as_str()
        .context("captured response URL")?;
    let value = serde_json::json!({
        "request": {"url": source_url, "semantic_url": source_url, "body": body},
        "receipt": response, "code": null, "status": response["http_status"],
        "message": "", "retryable": false, "retry_after_ms": 0
    });
    Ok(store
        .record_attempt(&operation, &value)?
        .as_str()
        .to_owned())
}

fn hydrate_operations(
    store: &ArtifactStore,
    operations: &mut [Value],
    responses: &[Value],
    known: &mut BTreeMap<String, String>,
    namespace: &EvidenceDigest,
) -> Result<()> {
    operations
        .iter_mut()
        .zip(responses)
        .try_for_each(|(operation, response)| {
            let source_url = response["source_url"].as_str().context("operation URL")?;
            let operation_id = fingerprint(&(namespace, source_url))?;
            let digest = match known.get(operation_id.as_str()) {
                Some(value) => value.clone(),
                None => {
                    let value = captured_attempt(store, operation_id.as_str(), response, None)?;
                    known.insert(operation_id.as_str().to_owned(), value.clone());
                    value
                }
            };
            operation["operation"] = serde_json::json!(operation_id);
            operation["attempts"] = serde_json::json!([digest]);
            operation["observed_attempts"] = serde_json::json!(1);
            operation["maximum_retries"] = serde_json::json!(3);
            operation["ownership"] = serde_json::json!("sdk_controlled");
            Ok::<(), anyhow::Error>(())
        })
}

fn hydrate_queries(
    store: &ArtifactStore,
    source: &athletic_rust_pipeline::model::SourceRecord,
    candidates: &[AthleteId],
) -> Result<Vec<String>> {
    query_plan(source)?
        .into_iter()
        .enumerate()
        .map(|(index, query)| {
            let suffix = match query.sport() {
                athletic_rust_pipeline::domain::evidence::Sport::TrackField => "track-and-field",
                athletic_rust_pipeline::domain::evidence::Sport::CrossCountry => "cross-country",
            };
            let result = candidates
                .iter()
                .map(|id| {
                    let id = id.get();
                    format!(
                    "<tr><td><a href=\"/athlete/{id}/{suffix}/all\">Synthetic Runner</a></td></tr>"
                )
                })
                .collect::<String>();
            let count = u32::try_from(candidates.len()).context("fixture candidate count")?;
            let body = serde_json::json!({"d":{"count":count,"pager":"","results":result}});
            let bytes = serde_json::to_vec(&body)?;
            let receipt_digest = store.put_bytes(&bytes)?.as_str().to_owned();
            let response = serde_json::json!({
                "digest":receipt_digest,
                "source_url":"http://127.0.0.1:18087/Search.aspx/runSearch",
                "http_status":200,"media_type":"application/json",
                "bytes":bytes.len(),"fetched_at_unix_ms":1,"elapsed_ms":1
            });
            let operation = fingerprint(&("query", &source.source_key, index))?;
            let attempt = captured_attempt(
                store,
                operation.as_str(),
                &response,
                Some(serde_json::json!({
                    "q":query.text(),"fq":format!("t:a {}",query.filter()),"start":0
                })),
            )?;
            let parsed = parse_page(&query, 0, EvidenceDigest::parse(&receipt_digest)?, &bytes)?;
            let page = QueryPage {
                response: serde_json::from_value(response)?,
                parsed: store.put_bytes(&serde_json::to_vec(&parsed)?)?,
                retries: serde_json::from_value(serde_json::json!({
                    "ownership":"sdk_controlled","operation":operation,
                    "maximum_retries":3,"observed_attempts":1,"attempts":[attempt]
                }))?,
                previous_responses: Vec::new(),
            };
            let evidence = QueryEvidence {
                query,
                pages: vec![page],
                complete: true,
                issues: Vec::new(),
                failures: Vec::new(),
            };
            let digest = store.put_bytes(&serde_json::to_vec(&evidence)?)?;
            Ok::<String, anyhow::Error>(digest.as_str().to_owned())
        })
        .collect()
}

fn hydrate_witness(row: &mut Value, store: &ArtifactStore) -> Result<()> {
    let team = br#"{"team":{"ID":501,"Name":"Central","State":"TX","Level":4}}"#;
    let team_digest = store.put_bytes(team)?.as_str().to_owned();
    remap_team_digest(row, &team_digest);
    let snapshot = SourceSnapshot {
        revision: athletic_rust_pipeline::runtime::acquisition::ACQUISITION_REVISION.to_owned(),
        source_origin: "http://127.0.0.1:18087/".to_owned(),
        label: "fuzz-retained-results".to_owned(),
    };
    let snapshot_digest = store
        .put_bytes(&serde_json::to_vec(&snapshot)?)?
        .as_str()
        .to_owned();
    row["report"]["job"]["snapshot"] = serde_json::json!(snapshot_digest);
    let source: athletic_rust_pipeline::model::SourceRecord =
        serde_json::from_value(row["source"].clone())?;
    let namespace = fingerprint(&source.source_key)?;
    let mut known = BTreeMap::new();
    ["profile_artifacts", "identity_artifacts"]
        .into_iter()
        .try_for_each(|field| {
            row[field]
                .as_array_mut()
                .context(field)?
                .iter_mut()
                .try_for_each(|artifact| {
                    for response in artifact["responses"].as_array_mut().context("responses")? {
                        if response["digest"].as_str() == Some(team_digest.as_str()) {
                            response["bytes"] = serde_json::json!(team.len());
                        }
                    }
                    let responses = artifact["responses"]
                        .as_array()
                        .context("responses")?
                        .clone();
                    let operations = artifact["operations"]
                        .as_array_mut()
                        .context("operations")?;
                    hydrate_operations(store, operations, &responses, &mut known, &namespace)
                })
        })?;
    let candidates: Vec<AthleteId> =
        serde_json::from_value(row["discovery"]["candidate_ids"].clone())?;
    let query_artifacts = hydrate_queries(store, &source, &candidates)?;
    row["discovery"]["job"] = row["report"]["job"].clone();
    row["discovery"]["query_artifacts"] = serde_json::json!(query_artifacts);
    let discovery: DiscoverySummary = serde_json::from_value(row["discovery"].clone())?;
    let discovery_digest = store
        .put_bytes(&serde_json::to_vec(&discovery)?)?
        .as_str()
        .to_owned();
    row["report"]["discovery"] = serde_json::json!(discovery_digest);
    row["report"]["query_evidence"] = serde_json::json!(discovery.query_artifacts);
    if row["assessment"].is_object() {
        let assessment =
            assessment::recompute(row, &source, EvidenceDigest::parse(&discovery_digest)?)?;
        row["assessment"] = serde_json::to_value(&assessment)?;
        let digest = store.put_bytes(&serde_json::to_vec(&assessment)?)?;
        row["report"]["assessment"] = serde_json::json!(digest.as_str());
    }
    let profile_digests = row["profile_artifacts"]
        .as_array()
        .context("profiles")?
        .iter()
        .map(|value| {
            let typed: ProfileAcquisition = serde_json::from_value(value.clone())?;
            Ok::<String, anyhow::Error>(
                store
                    .put_bytes(&serde_json::to_vec(&typed)?)?
                    .as_str()
                    .to_owned(),
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let probe_digests = row["identity_artifacts"]
        .as_array()
        .context("probes")?
        .iter()
        .map(|value| {
            let typed: ProfileProbe = serde_json::from_value(value.clone())?;
            Ok::<String, anyhow::Error>(
                store
                    .put_bytes(&serde_json::to_vec(&typed)?)?
                    .as_str()
                    .to_owned(),
            )
        })
        .collect::<Result<Vec<_>>>()?;
    row["report"]["candidates"]
        .as_array_mut()
        .context("candidate coverage")?
        .iter_mut()
        .enumerate()
        .for_each(|(index, candidate)| {
            let Some(kind) = candidate.as_object_mut() else {
                return;
            };
            let Some(object) = kind.values_mut().next().and_then(Value::as_object_mut) else {
                return;
            };
            if let Some(value) = probe_digests.get(index) {
                object.insert("probe".to_owned(), serde_json::json!(value));
            }
            if let Some(value) = profile_digests.get(index) {
                object.insert("profile".to_owned(), serde_json::json!(value));
            }
        });
    let report: RowReport = serde_json::from_value(row["report"].clone())?;
    let digest = store.put_bytes(&serde_json::to_vec(&report)?)?;
    row["report_digest"] = serde_json::json!(digest.as_str());
    Ok(())
}

const TRUSTED_RAW_DOCUMENTS: [(&str, &[u8]); 7] = [
    (
        "2a6e5945f027e7b76ead1f17dc994964d75e664a83ecca8f56b9665a1ef6693e",
        include_bytes!("../fixtures/retained_results_jsonl/raw/2a6e5945f027e7b76ead1f17dc994964d75e664a83ecca8f56b9665a1ef6693e"),
    ),
    (
        "386be9de58997ccd1906f54ce259f1a5265eace112fc02abeedafaa7dc3fa922",
        include_bytes!("../fixtures/retained_results_jsonl/raw/386be9de58997ccd1906f54ce259f1a5265eace112fc02abeedafaa7dc3fa922"),
    ),
    (
        "b666df3f8d4b132a278ceeebad16ac11ecbe4c6c83c8f493c70268a187e176fe",
        include_bytes!("../fixtures/retained_results_jsonl/raw/b666df3f8d4b132a278ceeebad16ac11ecbe4c6c83c8f493c70268a187e176fe"),
    ),
    (
        "c6394fc995337757729f9eedfbb6861df62f05e756913388c28cfa7ebb945864",
        include_bytes!("../fixtures/retained_results_jsonl/raw/c6394fc995337757729f9eedfbb6861df62f05e756913388c28cfa7ebb945864"),
    ),
    (
        "2fc43c65625879c99e34adff06876c86bcfd685073ea9d23cf84b0beb798f531",
        include_bytes!("../fixtures/retained_results_jsonl/raw/2fc43c65625879c99e34adff06876c86bcfd685073ea9d23cf84b0beb798f531"),
    ),
    (
        "37b72516909fc95821215c5f9ec1ce40e1d696c613dad8f96a22e140ca477560",
        include_bytes!("../fixtures/retained_results_jsonl/raw/37b72516909fc95821215c5f9ec1ce40e1d696c613dad8f96a22e140ca477560"),
    ),
    (
        "6bfbe82b524a8a7a14fe4d286e22b154d99f33baf77e43e96cba03ebd6cf1ae3",
        include_bytes!("../fixtures/retained_results_jsonl/raw/6bfbe82b524a8a7a14fe4d286e22b154d99f33baf77e43e96cba03ebd6cf1ae3"),
    ),
];

fn seed_raw_documents(store: &ArtifactStore) -> Result<()> {
    TRUSTED_RAW_DOCUMENTS
        .iter()
        .try_for_each(|(expected, bytes)| {
            let expected = EvidenceDigest::parse(expected).context("invalid trusted raw digest")?;
            let actual = store
                .put_bytes(bytes)
                .context("storing trusted raw document")?;
            if actual != expected {
                bail!("trusted raw document digest differs from manifest");
            }
            Ok(())
        })
}

fn verify_bytes(data: &[u8]) -> Result<ResultVerificationReport> {
    let fixtures = trusted_store()?;
    let bound = references::bind(data, &fixtures.references);
    let mut file = NamedTempFile::new().context("creating temporary JSONL")?;
    file.write_all(&bound).context("writing temporary JSONL")?;
    verify_results(file.path(), &fixtures.store)
}

fn hydrated_witness(index: usize) -> Result<Vec<u8>> {
    match FIXTURE_STORE.get() {
        Some(Ok(fixtures)) => fixtures
            .witnesses
            .get(index)
            .cloned()
            .context("hydrated witness index"),
        Some(Err(error)) => bail!("trusted fixture store failed: {error}"),
        None => bail!("trusted fixture store was not initialized"),
    }
}

fn check_witnesses() {
    if let Err(error) = trusted_store() {
        panic!("trusted fixture store unavailable: {error:#}");
    }
    let valid = [
        ([1, 1, 0, 0, 0], 0_usize),
        ([1, 0, 0, 1, 0], 1_usize),
        ([1, 0, 0, 0, 1], 3_usize),
        ([1, 0, 0, 1, 0], 2_usize),
    ];
    valid.into_iter().for_each(|(expected, index)| {
        let data = if index == 3 {
            include_bytes!("../fixtures/retained_results_jsonl/seed-valid-pending.json").to_vec()
        } else {
            match hydrated_witness(index) {
                Ok(data) => data,
                Err(error) => panic!("hydrated witness unavailable: {error:#}"),
            }
        };
        let report = match verify_bytes(&data) {
            Ok(report) => report,
            Err(error) => panic!("valid retained-results witness rejected: {error:#}"),
        };
        assert_eq!(counts(&report), expected);
        if let Some(original) = TRUSTED_WITNESSES.get(index) {
            let original_report = match verify_bytes(original) {
                Ok(report) => report,
                Err(error) => panic!("committed retained-results seed rejected: {error:#}"),
            };
            assert_eq!(counts(&original_report), expected);
        }
    });
    [
        include_bytes!("../fixtures/retained_results_jsonl/seed-unknown-athlete-id.json")
            .as_slice(),
        include_bytes!("../fixtures/retained_results_jsonl/seed-hard-contradiction.json")
            .as_slice(),
        include_bytes!("../fixtures/retained_results_jsonl/seed-duplicate-field.json").as_slice(),
        include_bytes!("../fixtures/retained_results_jsonl/seed-malformed.json").as_slice(),
    ]
    .into_iter()
    .for_each(|data| {
        assert!(
            verify_bytes(data).is_err(),
            "invalid retained-results witness accepted"
        );
    });
}

fn counts(report: &ResultVerificationReport) -> [u64; 5] {
    [
        report.total_rows,
        report.accepted_rows,
        report.review_rows,
        report.no_match_rows,
        report.pending_rows,
    ]
}

fuzz_target!(|data: &[u8]| {
    WITNESSES.call_once(check_witnesses);
    if data.len() > MAX_INPUT_BYTES {
        return;
    }
    if let Ok(report) = verify_bytes(data) {
        assert_eq!(
            report.total_rows,
            report.accepted_rows + report.review_rows + report.no_match_rows + report.pending_rows
        );
        let row_count = match u64::try_from(data.iter().filter(|byte| **byte == b'\n').count()) {
            Ok(count) => count,
            Err(error) => panic!("bounded input row count overflow: {error}"),
        };
        assert_eq!(report.total_rows, row_count);
    }
});
