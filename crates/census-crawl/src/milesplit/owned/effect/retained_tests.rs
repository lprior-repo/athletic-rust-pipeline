use super::*;
use crate::milesplit::owned::{
    read_retained_acquisition, CaptureReadBudget, CaptureWitnessError, OwnedAcquisitionRef,
    MAX_OWNED_BODY_BYTES,
};
use crate::net::cache::content_digest;
use crate::net::Fetcher;
use census_domain::model::SchoolYear;
use census_store::Store;
use serde_json::{json, Value};
use std::collections::HashMap;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

mod corruptions;
mod ownership;
mod positives;
mod references;

fn source() -> TestResult<ResultSetRef> {
    ResultSetRef::parse("https://al.milesplit.com/meets/725218/results/1266814/raw")
        .ok_or_else(|| "test source reference".into())
}

fn sample(body: &[u8], status: u16) -> TestResult<FetchOutcome> {
    Ok(FetchOutcome {
        url: owned_meet_url(&source()?)?,
        response_url: None,
        method: "GET".into(),
        status,
        content_digest: content_digest(body),
        bytes: body.len(),
        fetched_at: "2026-10-01T23:44:16Z".into(),
        from_cache: false,
        content_type: Some("application/json".into()),
        body: body.to_vec(),
    })
}

fn archive(store: &Store, capture: &FetchOutcome) -> TestResult<OwnedAcquisitionRef> {
    let dir = tempfile::tempdir()?;
    let fetcher = Fetcher::new(
        dir.path().join("offline"),
        None,
        std::time::Duration::ZERO,
        HashMap::new(),
        vec!["milesplit.com".into()],
    )?
    .with_offline(true);
    let ctx = AdapterContext {
        capture: crate::CaptureContext::LegacyMetadata,
        fetcher: &fetcher,
        store,
        refresh: false,
        school_year: SchoolYear::new(2026).ok_or("school year")?,
        observed_on: "2099-01-01".into(),
        performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 10, 1).ok_or("as-of date")?,
        recording: None,
    };
    let reference = source()?;
    let key = acquisition_manifest_key(725218, capture)?;
    capture::archive(&ctx, &content_key(&reference, capture), &key, capture)?;
    Ok(OwnedAcquisitionRef::parse(&key)?)
}

fn fixture(
    body: &[u8],
) -> TestResult<(tempfile::TempDir, Store, FetchOutcome, OwnedAcquisitionRef)> {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    let capture = sample(body, 200)?;
    let reference = archive(&store, &capture)?;
    Ok((dir, store, capture, reference))
}

fn budget() -> TestResult<CaptureReadBudget> {
    Ok(CaptureReadBudget::new(MAX_OWNED_BODY_BYTES)?)
}

fn payload(store: &Store, key: &str) -> TestResult<Value> {
    store
        .journal_payload(OWNED_CAPTURE_PHASE, key)?
        .ok_or_else(|| "retained payload".into())
}

fn replace(store: &Store, key: &str, value: &Value) -> TestResult {
    let mut batch = store.write_batch();
    batch.journal_done(OWNED_CAPTURE_PHASE, key, value)?;
    batch.commit()?;
    Ok(())
}

fn chunk_key(capture: &FetchOutcome, index: usize) -> String {
    format!("725218/{}/{index}", capture.content_digest)
}

fn content_manifest(capture: &FetchOutcome) -> String {
    format!("725218/{}/manifest", capture.content_digest)
}

fn copy_except(source: &Store, target: &Store, omitted: &str) -> TestResult {
    let mut batch = target.write_batch();
    for key in source.journal_keys(OWNED_CAPTURE_PHASE)? {
        if key != omitted {
            batch.journal_done(OWNED_CAPTURE_PHASE, &key, &payload(source, &key)?)?;
        }
    }
    batch.commit()?;
    Ok(())
}

fn require_corrupt(
    result: Result<super::super::VerifiedOwnedAcquisition, CaptureWitnessError>,
    key: &str,
) -> TestResult {
    match result {
        Err(CaptureWitnessError::Corrupt { key: found, detail }) => {
            check!(eq; found, key);
            check!(!detail.is_empty());
            Ok(())
        }
        found => Err(format!("expected corrupt {key}, got {found:?}").into()),
    }
}

fn require_foreign(
    result: Result<super::super::VerifiedOwnedAcquisition, CaptureWitnessError>,
    key: &str,
) -> TestResult {
    match result {
        Err(CaptureWitnessError::Foreign { key: found, detail }) => {
            check!(eq; found, key);
            check!(!detail.is_empty());
            Ok(())
        }
        found => Err(format!("expected foreign {key}, got {found:?}").into()),
    }
}
