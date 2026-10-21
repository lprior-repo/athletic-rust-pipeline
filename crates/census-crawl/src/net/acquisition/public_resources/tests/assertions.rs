use super::*;
use crate::net::CapturedFetch;
use census_store::{AcquisitionReadBudget, Receipt};

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Witness {
    sequence: u64,
    receipts: u64,
    keys: Vec<String>,
}

impl Witness {
    pub(super) fn capture(store: &Store) -> TestResult<Self> {
        let mut keys: Vec<_> = store
            .journal_keys(CaptureProtocolVersion::V2.phase())?
            .into_iter()
            .collect();
        check!(keys.len() <= 128);
        keys.sort();
        Ok(Self {
            sequence: store.snapshot().sequence(),
            receipts: store.receipt_count()?,
            keys,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Retained {
    occurrence: AcquisitionOccurrenceRefV1,
    metadata: Vec<u8>,
    body: Vec<u8>,
    observed: NormalizedObservedHttpV1,
    receipt: Receipt,
}

pub(super) fn retained(store: &Store, captured: &CapturedFetch) -> TestResult<Retained> {
    let view = captured
        .retained_observed()?
        .ok_or("owned observed capture absent")?;
    check!(eq; view.original(), view.occurrence());
    let held = store
        .read_retained_acquisition_v1(view.occurrence(), &mut AcquisitionReadBudget::for_work())?;
    check!(eq; held.prepared().metadata_bytes(), view.original_metadata());
    check!(eq; std::fs::read(held.body_path())?, captured.capture().body);
    let attempt = view.occurrence().attempt();
    let operation = format!(
        "{}/work/{}/attempt/{}/terminal",
        CaptureProtocolVersion::V2.phase(),
        attempt.work_id(),
        attempt.ordinal().get()
    );
    let receipt = store
        .receipt(&operation)?
        .ok_or("retained public-root terminal receipt")?;
    check!(eq; receipt.digest, view.occurrence().terminal_digest().as_str());
    check!(eq; receipt.appended, 0);
    check!(!receipt.credited);
    Ok(Retained {
        occurrence: view.occurrence().clone(),
        metadata: view.original_metadata().to_vec(),
        body: captured.capture().body.clone(),
        observed: view.observed_http().clone(),
        receipt,
    })
}

pub(super) fn exact_hop(
    captured: &CapturedFetch,
    index: usize,
    url: &str,
    status: u16,
    body_len: usize,
    content_type: &str,
) -> TestResult {
    let observed = captured
        .observed_http()?
        .ok_or("public-root HTTP observation")?;
    let hop = observed
        .hops()
        .get(index)
        .ok_or("expected public-root HTTP hop")?;
    check!(eq; hop.request().target_url(), url);
    check!(eq; hop.response().response_url(), url);
    check!(eq; hop.response().status(), status);
    check!(eq; hop.response().protocol(), ObservedHttpProtocolV1::Http11);
    let mut request_headers = headers(hop.request().headers());
    request_headers.sort();
    check!(eq; request_headers, vec![
        ("accept".into(), b"*/*".to_vec()),
        ("accept-encoding".into(), b"identity".to_vec()),
        ("user-agent".into(), b"census-service/0.1".to_vec()),
    ]);
    let response_headers = headers(hop.response().headers());
    for expected in [
        ("connection".into(), b"close".to_vec()),
        ("content-length".into(), body_len.to_string().into_bytes()),
        ("content-type".into(), content_type.as_bytes().to_vec()),
        ("etag".into(), b"\"public-root-v1\"".to_vec()),
        ("x-capture".into(), b"controlled".to_vec()),
    ] {
        check!(eq; response_headers.iter().filter(|pair| **pair == expected).count(), 1);
    }
    let started = chrono::DateTime::parse_from_rfc3339(hop.request().started_at())?;
    let received = chrono::DateTime::parse_from_rfc3339(hop.response().received_at())?;
    check!(started <= received);
    Ok(())
}

fn headers(headers: &[ObservedHttpHeaderV1]) -> Vec<(String, Vec<u8>)> {
    headers
        .iter()
        .map(|header| (header.name().to_owned(), header.value().to_vec()))
        .collect()
}

pub(super) fn no_directory_attempt(store: &Store, work: &AcquisitionWorkV1) -> TestResult {
    let keys = store.journal_keys(CaptureProtocolVersion::V2.phase())?;
    let prefix = format!("work/{}/attempt/", work.id()?);
    check!(keys.iter().all(|key| !key.starts_with(&prefix)));
    Ok(())
}
