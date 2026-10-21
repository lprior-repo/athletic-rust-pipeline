use super::*;
use sha2::{Digest, Sha256};

pub(super) fn prepare(
    f: &Fixture,
    owner: &PublicResourceCaptureOwnerV1,
    status: u16,
) -> TestResult<(crate::PreparedCapture, Vec<u8>)> {
    let work = owner.for_root(&f.store, PublicResourceRootKindV1::MileSplitTeamDirectory)?;
    let reserved = f.store.reserve_acquisition_attempt(registered(
        &f.store,
        &f.writer,
        owner,
        work.clone(),
    )?)?;
    let mut request = request(&work, 0)?;
    let mut armed = f
        .store
        .arm_acquisition_dispatch(reserved, request.clone())?;
    let mut hops = Vec::new();
    for hop in 0..=4 {
        let code = if hop == 4 { status } else { 302 };
        let response = response(work.request().original_url(), hop, code)?;
        hops.push(ObservedHttpHopV1::new(
            armed.dispatch_id().clone(),
            request
                .observed_http()
                .ok_or("observed public request")?
                .clone(),
            response
                .observed_http()
                .ok_or("observed public response")?
                .clone(),
        )?);
        let observed = f
            .store
            .record_acquisition_response(armed.into_observation(), response.clone())?;
        if hop == 4 {
            let body = format!("controlled public response {status}").into_bytes();
            let input = input(&work, &response, hops, &body)?;
            let prepared = f.store.prepare_acquisition_capture(
                observed,
                input,
                CaptureDispositionV1::for_http_status(status)?,
            )?;
            return Ok((prepared, body));
        }
        request = self::request(&work, hop.checked_add(1).ok_or("next hop")?)?;
        armed = f
            .store
            .arm_acquisition_redirect(observed, request.clone())?;
    }
    Err("five-hop public response absent".into())
}

fn request(work: &AcquisitionWorkV1, hop: u8) -> TestResult<DispatchRequestV1> {
    let headers = work
        .request()
        .representation_headers()
        .iter()
        .map(|(name, value)| ObservedHttpHeaderV1::new(name, value.as_bytes().to_vec()))
        .collect::<Result<Vec<_>, _>>()?;
    let seconds = hop.checked_mul(2).ok_or("request second")?;
    let time = format!("2026-10-14T05:00:{seconds:02}Z");
    Ok(DispatchRequestV1::new_observed(
        work.request().original_url(),
        ConditionalRequestV1::Unconditional,
        ObservedHttpRequestV1::new(work.request().original_url(), &time, headers)?,
    )?)
}

fn response(url: &str, hop: u8, status: u16) -> TestResult<ResponseObservationV1> {
    let seconds = hop
        .checked_mul(2)
        .and_then(|value| value.checked_add(1))
        .ok_or("response second")?;
    let time = format!("2026-10-14T05:00:{seconds:02}Z");
    let headers = if status == 302 {
        vec![("location".to_string(), url.to_string())]
    } else {
        vec![
            ("content-type".to_string(), "text/html".to_string()),
            ("etag".to_string(), "controlled-public-etag".to_string()),
        ]
    };
    let mut observed = headers
        .iter()
        .map(|(name, value)| ObservedHttpHeaderV1::new(name, value.as_bytes().to_vec()))
        .collect::<Result<Vec<_>, _>>()?;
    observed.push(ObservedHttpHeaderV1::new(
        "x-controlled-raw",
        vec![128, 255],
    )?);
    Ok(ResponseObservationV1::new_observed(
        status,
        url,
        &time,
        headers,
        ObservedHttpResponseV1::new(ObservedHttpProtocolV1::Http2, status, url, &time, observed)?,
    )?)
}

fn input(
    work: &AcquisitionWorkV1,
    response: &ResponseObservationV1,
    hops: Vec<ObservedHttpHopV1>,
    body: &[u8],
) -> TestResult<PreparedCaptureInputV1> {
    let digest = AcquisitionDigest::parse(&format!("{:x}", Sha256::digest(body)))?;
    let bytes = u64::try_from(body.len())?;
    let redirects = hops
        .iter()
        .take(4)
        .map(|hop| {
            serde_json::json!({
                "status": hop.response().status(), "url": work.request().original_url()
            })
        })
        .collect::<Vec<_>>();
    let normalized =
        NormalizedObservedHttpV1::new(hops, "2026-10-14T05:00:10Z", digest.clone(), bytes)?;
    let metadata = serde_json::to_vec_pretty(&serde_json::json!({
        "url": work.request().original_url(), "response_url": response.response_url(),
        "method": "GET", "representation": work.request().representation_headers(),
        "status": response.status(), "content_digest": digest, "bytes": bytes,
        "fetched_at": response.observed_at(), "etag": "controlled-public-etag",
        "last_modified": null, "content_type": "text/html", "redirects": redirects,
        "observed_http": normalized,
    }))?;
    Ok(PreparedCaptureInputV1::new(
        digest.as_str(),
        bytes,
        metadata,
    )?)
}

pub(super) fn archive_pair(
    store: &Store,
    input: &PreparedCaptureInputV1,
) -> TestResult<(Vec<u8>, Vec<u8>)> {
    let body = store
        .http_cache_dir()
        .join("archive/bodies")
        .join(format!("{}.body", input.content_digest()));
    let metadata = store
        .http_cache_dir()
        .join("archive/captures")
        .join(input.content_digest().as_str())
        .join(format!("{}.meta.json", input.metadata_digest()));
    Ok((std::fs::read(body)?, std::fs::read(metadata)?))
}
