use super::*;
use sha2::{Digest, Sha256};

const START: &str = "2026-10-14T05:00:00Z";
const RECEIVED: &str = "2026-10-14T05:00:01Z";
const DONE: &str = "2026-10-14T05:00:02Z";

pub(super) fn prepare(
    store: &Store,
    writer: &CaptureWriterV1,
    owner: &PublicResourceCaptureOwnerV1,
) -> TestResult<(crate::PreparedCapture, Vec<u8>)> {
    let work = owner.for_root(store, PublicResourceRootKindV1::MileSplitTeamDirectory)?;
    let owned = registered(store, writer, owner, work)?;
    prepare_work(
        store,
        owned,
        b"<html><body>controlled directory capture</body></html>".to_vec(),
    )
}

pub(super) fn prepare_work(
    store: &Store,
    owned: OwnedAcquisitionWork,
    body: Vec<u8>,
) -> TestResult<(crate::PreparedCapture, Vec<u8>)> {
    let work = owned.work().clone();
    let headers = work
        .request()
        .representation_headers()
        .iter()
        .map(|(name, value)| ObservedHttpHeaderV1::new(name, value.as_bytes().to_vec()))
        .collect::<Result<Vec<_>, _>>()?;
    let request = DispatchRequestV1::new_observed(
        work.request().original_url(),
        ConditionalRequestV1::Unconditional,
        ObservedHttpRequestV1::new(work.request().original_url(), START, headers)?,
    )?;
    let reserved = store.reserve_acquisition_attempt(owned)?;
    let armed = store.arm_acquisition_dispatch(reserved, request.clone())?;
    let response = response(work.request().original_url())?;
    let input = input(
        &work,
        armed.dispatch_id().clone(),
        &request,
        &response,
        &body,
    )?;
    let observed = store.record_acquisition_response(armed.into_observation(), response)?;
    let prepared = store.prepare_acquisition_capture(
        observed,
        input,
        CaptureDispositionV1::for_http_status(200)?,
    )?;
    Ok((prepared, body))
}

fn response(url: &str) -> TestResult<ResponseObservationV1> {
    Ok(ResponseObservationV1::new_observed(
        200,
        url,
        RECEIVED,
        vec![
            ("content-type".to_string(), "text/html".to_string()),
            ("etag".to_string(), "controlled-etag".to_string()),
        ],
        ObservedHttpResponseV1::new(
            ObservedHttpProtocolV1::Http2,
            200,
            url,
            RECEIVED,
            vec![
                ObservedHttpHeaderV1::new("content-type", b"text/html".to_vec())?,
                ObservedHttpHeaderV1::new("etag", b"controlled-etag".to_vec())?,
                ObservedHttpHeaderV1::new("x-source", vec![128])?,
            ],
        )?,
    )?)
}

fn input(
    work: &AcquisitionWorkV1,
    dispatch: AcquisitionDispatchId,
    request: &DispatchRequestV1,
    response: &ResponseObservationV1,
    body: &[u8],
) -> TestResult<PreparedCaptureInputV1> {
    let digest = AcquisitionDigest::parse(&format!("{:x}", Sha256::digest(body)))?;
    let bytes = u64::try_from(body.len())?;
    let normalized = NormalizedObservedHttpV1::new(
        vec![ObservedHttpHopV1::new(
            dispatch,
            request
                .observed_http()
                .ok_or("actual observed request")?
                .clone(),
            response
                .observed_http()
                .ok_or("actual observed response")?
                .clone(),
        )?],
        DONE,
        digest.clone(),
        bytes,
    )?;
    let metadata = serde_json::to_vec_pretty(&serde_json::json!({
        "url": work.request().original_url(),
        "response_url": response.response_url(),
        "method": "GET",
        "representation": work.request().representation_headers(),
        "status": response.status(),
        "content_digest": digest,
        "bytes": bytes,
        "fetched_at": response.observed_at(),
        "etag": "controlled-etag",
        "last_modified": null,
        "content_type": "text/html",
        "redirects": [],
        "observed_http": normalized,
    }))?;
    Ok(PreparedCaptureInputV1::new(
        digest.as_str(),
        bytes,
        metadata,
    )?)
}

pub(super) fn publish(store: &Store, capture: &PreparedCaptureInputV1, body: &[u8]) -> TestResult {
    let bodies = store.http_cache_dir().join("archive/bodies");
    let captures = store
        .http_cache_dir()
        .join("archive/captures")
        .join(capture.content_digest().as_str());
    crate::fs::create_dir_all_synced(&bodies)?;
    crate::fs::create_dir_all_synced(&captures)?;
    std::fs::write(
        bodies.join(format!("{}.body", capture.content_digest())),
        body,
    )?;
    std::fs::write(
        captures.join(format!("{}.meta.json", capture.metadata_digest())),
        capture.metadata_bytes(),
    )?;
    Ok(())
}
