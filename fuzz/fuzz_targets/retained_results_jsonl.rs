#![no_main]

use anyhow::{bail, Context, Result};
use athletic_rust_pipeline::{
    domain::{
        decision::{CandidateReason, Decision, SearchCompleteness},
        identity::{AthleteId, EvidenceDigest},
    },
    result_verify::{verify_results, ResultVerificationReport},
    runtime::{
        acquisition::{ProfileAcquisition, ProfileProbe},
        row_protocol::{DiscoverySummary, RowReport},
    },
    store::ArtifactStore,
};
use libfuzzer_sys::fuzz_target;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
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

fn trusted_store() -> Result<&'static ArtifactStore> {
    match FIXTURE_STORE.get_or_init(|| build_fixture_store().map_err(|error| format!("{error:#}")))
    {
        Ok(fixtures) => Ok(&fixtures.store),
        Err(error) => bail!("trusted retained-results fixtures are invalid: {error}"),
    }
}

fn build_fixture_store() -> Result<FixtureStore> {
    let root = tempdir().context("creating retained-results fixture directory")?;
    let store = ArtifactStore::open(&root.path().join("artifacts"))
        .context("opening retained-results fixture store")?;
    TRUSTED_WITNESSES
        .iter()
        .try_for_each(|bytes| seed_witness(&store, bytes))?;
    seed_raw_documents(&store)?;
    Ok(FixtureStore { _root: root, store })
}

fn seed_witness(store: &ArtifactStore, bytes: &[u8]) -> Result<()> {
    let row: Value = serde_json::from_slice(bytes).context("decoding trusted witness")?;
    let Some(report_value) = row.get("report").filter(|value| !value.is_null()) else {
        return Ok(());
    };
    let report: RowReport =
        serde_json::from_value(report_value.clone()).context("decoding trusted row report")?;
    let report_digest = digest_field(&row, "report_digest")?;
    seed_typed::<RowReport>(store, report_value, &report_digest, "row report")?;

    let discovery_digest = report
        .discovery
        .as_ref()
        .context("trusted report has no discovery digest")?;
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

    let assessment_digest = report
        .assessment
        .as_ref()
        .context("trusted report has no assessment digest")?;
    let assessment_value = row
        .get("assessment")
        .filter(|value| !value.is_null())
        .context("trusted report has no assessment artifact")?;
    seed_typed::<AssessmentWire>(store, assessment_value, assessment_digest, "assessment")?;

    let identity_values = row
        .get("identity_artifacts")
        .and_then(Value::as_array)
        .context("trusted identity artifacts are not an array")?;
    let profile_values = row
        .get("profile_artifacts")
        .and_then(Value::as_array)
        .context("trusted profile artifacts are not an array")?;
    let mut identities = identity_values.iter();
    let mut profiles = profile_values.iter();
    report.candidates.iter().try_for_each(|candidate| {
        if let Some(digest) = candidate.probe() {
            let value = identities
                .next()
                .context("trusted probe artifact is missing")?;
            seed_typed::<ProfileProbe>(store, value, digest, "profile probe")?;
        }
        if let Some(digest) = candidate.profile() {
            let value = profiles
                .next()
                .context("trusted profile acquisition is missing")?;
            seed_typed::<ProfileAcquisition>(store, value, digest, "profile acquisition")?;
        }
        Ok::<(), anyhow::Error>(())
    })?;
    if identities.next().is_some() || profiles.next().is_some() {
        bail!("trusted witness carries unreferenced identity artifacts");
    }
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
    let mut file = NamedTempFile::new().context("creating temporary JSONL")?;
    file.write_all(data).context("writing temporary JSONL")?;
    let store = trusted_store()?;
    Ok(verify_results(file.path(), store)?)
}

fn check_witnesses() {
    for (data, expected) in [
        (
            include_bytes!("../fixtures/retained_results_jsonl/seed-valid-accepted.json")
                .as_slice(),
            [1, 1, 0, 0, 0],
        ),
        (
            include_bytes!("../fixtures/retained_results_jsonl/seed-valid-no-match.json")
                .as_slice(),
            [1, 0, 0, 1, 0],
        ),
        (
            include_bytes!("../fixtures/retained_results_jsonl/seed-valid-pending.json").as_slice(),
            [1, 0, 0, 0, 1],
        ),
        (
            include_bytes!("../fixtures/retained_results_jsonl/seed-valid-excluded.json")
                .as_slice(),
            [1, 0, 0, 1, 0],
        ),
    ] {
        let report = match verify_bytes(data) {
            Ok(report) => report,
            Err(error) => panic!("valid retained-results witness rejected: {error:#}"),
        };
        assert_eq!(counts(&report), expected);
    }
    for data in [
        include_bytes!("../fixtures/retained_results_jsonl/seed-unknown-athlete-id.json")
            .as_slice(),
        include_bytes!("../fixtures/retained_results_jsonl/seed-hard-contradiction.json")
            .as_slice(),
        include_bytes!("../fixtures/retained_results_jsonl/seed-duplicate-field.json").as_slice(),
        include_bytes!("../fixtures/retained_results_jsonl/seed-malformed.json").as_slice(),
    ] {
        assert!(
            verify_bytes(data).is_err(),
            "invalid retained-results witness accepted"
        );
    }
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
