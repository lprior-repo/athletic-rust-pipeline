use super::*;

pub(super) const URL: &str = "https://example.test/source";
pub(super) const DONE: &str = "2026-10-21T08:00:12Z";

pub(super) fn digest() -> TestResult<AcquisitionDigest> {
    Ok(AcquisitionDigest::parse(&"a".repeat(64))?)
}

pub(super) fn dispatch(hop: u8, attempt: u8) -> TestResult<AcquisitionDispatchId> {
    Ok(AcquisitionDispatchId::new(
        AcquisitionAttemptId::new(
            StoreIdentity::parse(&"A".repeat(64))?,
            AcquisitionWorkId::parse(&"b".repeat(64))?,
            AttemptOrdinal::new(attempt)?,
        ),
        HopOrdinal::new(hop)?,
    ))
}

pub(super) fn response(headers: Vec<ObservedHttpHeaderV1>) -> TestResult<ObservedHttpResponseV1> {
    Ok(ObservedHttpResponseV1::new(
        ObservedHttpProtocolV1::Http2,
        200,
        URL,
        "2026-10-21T08:00:01Z",
        headers,
    )?)
}

pub(super) fn hop(
    ordinal: u8,
    request_headers: Vec<ObservedHttpHeaderV1>,
    response_headers: Vec<ObservedHttpHeaderV1>,
) -> TestResult<ObservedHttpHopV1> {
    let start = ordinal.checked_mul(2).ok_or("fixture clock overflow")?;
    let head = start.checked_add(1).ok_or("fixture clock overflow")?;
    Ok(ObservedHttpHopV1::new(
        dispatch(ordinal, 1)?,
        ObservedHttpRequestV1::new(
            URL,
            &format!("2026-10-21T08:00:{start:02}Z"),
            request_headers,
        )?,
        ObservedHttpResponseV1::new(
            ObservedHttpProtocolV1::Http2,
            200,
            URL,
            &format!("2026-10-21T08:00:{head:02}Z"),
            response_headers,
        )?,
    )?)
}

pub(super) fn observed(hops: Vec<ObservedHttpHopV1>) -> TestResult<NormalizedObservedHttpV1> {
    Ok(NormalizedObservedHttpV1::new(hops, DONE, digest()?, 4)?)
}

pub(super) fn original() -> TestResult<NormalizedObservedHttpV1> {
    observed(vec![hop(0, Vec::new(), Vec::new())?])
}

pub(super) fn redirect(hop: ObservedHttpHopV1, target: &str) -> TestResult<ObservedHttpHopV1> {
    let mut headers = hop.response().headers().to_vec();
    headers.push(ObservedHttpHeaderV1::new(
        "location",
        target.as_bytes().to_vec(),
    )?);
    let response = ObservedHttpResponseV1::new(
        hop.response().protocol(),
        302,
        hop.response().response_url(),
        hop.response().received_at(),
        headers,
    )?;
    Ok(ObservedHttpHopV1::new(
        hop.dispatch().clone(),
        hop.request().clone(),
        response,
    )?)
}

pub(super) fn chain(count: u8) -> TestResult<Vec<ObservedHttpHopV1>> {
    (0..count)
        .map(|ordinal| {
            let observed = hop(ordinal, Vec::new(), Vec::new())?;
            if ordinal.checked_add(1).ok_or("fixture ordinal")? < count {
                redirect(observed, URL)
            } else {
                Ok(observed)
            }
        })
        .collect()
}

pub(super) fn headers(
    count: usize,
    length: usize,
    byte: u8,
) -> TestResult<Vec<ObservedHttpHeaderV1>> {
    (0..count)
        .map(|_| Ok(ObservedHttpHeaderV1::new("x", vec![byte; length])?))
        .collect()
}

pub(super) fn request_headers(value: &mut serde_json::Value) -> TestResult<&mut serde_json::Value> {
    value
        .pointer_mut("/hops/0/request/headers")
        .ok_or_else(|| "fixture headers missing".into())
}
