use super::*;
use census_store::{AcquisitionReadBudget, PreparedCapture};

const START: &str = "2026-10-14T05:00:00Z";
const HEAD: &str = "2026-10-14T05:00:01Z";
const DONE: &str = "2026-10-14T05:00:02Z";

pub(super) fn retain(
    store: &Store,
    writer: &CaptureWriterV1,
    owner: &PublicResourceCaptureOwnerV1,
    body: &[u8],
    status: u16,
) -> TestResult<AcquisitionOccurrenceRefV1> {
    let work = owner.for_root(store, PublicResourceRootKindV1::MileSplitTeamDirectory)?;
    let request = request(&work)?;
    let grant = writer.continuation_for_public_resource(store, owner, &work)?;
    let owned = store.register_acquisition_work(owner, work.clone(), grant)?;
    let reserved = store.reserve_acquisition_attempt(owned)?;
    let armed = store.arm_acquisition_dispatch(reserved, request.clone())?;
    let dispatch = armed.dispatch_id().clone();
    let response = response(status)?;
    let input = input(&work, dispatch, &request, &response, body)?;
    let observed = store.record_acquisition_response(armed.into_observation(), response)?;
    let prepared = store.prepare_acquisition_capture(
        observed,
        input,
        CaptureDispositionV1::for_http_status(status)?,
    )?;
    publish(store, &prepared, body)?;
    let reference = store.finish_acquisition_capture(prepared)?;
    store.read_retained_acquisition_v1(&reference, &mut AcquisitionReadBudget::for_work())?;
    Ok(reference)
}

fn request(work: &AcquisitionWorkV1) -> TestResult<DispatchRequestV1> {
    let headers = work
        .request()
        .representation_headers()
        .iter()
        .map(|(name, value)| ObservedHttpHeaderV1::new(name, value.as_bytes().to_vec()))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(DispatchRequestV1::new_observed(
        URL,
        ConditionalRequestV1::Unconditional,
        ObservedHttpRequestV1::new(URL, START, headers)?,
    )?)
}

fn response(status: u16) -> TestResult<ResponseObservationV1> {
    Ok(ResponseObservationV1::new_observed(
        status,
        URL,
        HEAD,
        vec![("content-type".into(), "text/html".into())],
        ObservedHttpResponseV1::new(
            ObservedHttpProtocolV1::Http2,
            status,
            URL,
            HEAD,
            vec![
                ObservedHttpHeaderV1::new("content-type", b"text/html".to_vec())?,
                ObservedHttpHeaderV1::new(
                    "x-controlled-transport",
                    b"no-original-provenance".to_vec(),
                )?,
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
    let digest = AcquisitionDigest::parse(&crate::net::cache::content_digest(body))?;
    let bytes = u64::try_from(body.len())?;
    let normalized = NormalizedObservedHttpV1::new(
        vec![ObservedHttpHopV1::new(
            dispatch,
            request.observed_http().ok_or("controlled request")?.clone(),
            response
                .observed_http()
                .ok_or("controlled response")?
                .clone(),
        )?],
        DONE,
        digest.clone(),
        bytes,
    )?;
    let metadata = serde_json::to_vec(&serde_json::json!({
        "url": URL,
        "response_url": URL,
        "method": "GET",
        "representation": work.request().representation_headers(),
        "status": response.status(),
        "content_digest": digest,
        "bytes": bytes,
        "fetched_at": HEAD,
        "etag": null,
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

fn publish(store: &Store, prepared: &PreparedCapture, body: &[u8]) -> TestResult {
    let capture = prepared.prepared();
    let bodies = store.http_cache_dir().join("archive/bodies");
    let captures = store
        .http_cache_dir()
        .join("archive/captures")
        .join(capture.content_digest().as_str());
    census_store::fs::create_dir_all_synced(&bodies)?;
    census_store::fs::create_dir_all_synced(&captures)?;
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
