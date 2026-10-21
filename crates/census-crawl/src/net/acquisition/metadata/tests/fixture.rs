use super::*;

fn dispatch(hop: u8) -> TestResult<AcquisitionDispatchId> {
    Ok(AcquisitionDispatchId::new(
        AcquisitionAttemptId::new(
            StoreIdentity::parse(&"A".repeat(64))?,
            AcquisitionWorkId::parse(&"b".repeat(64))?,
            AttemptOrdinal::new(1)?,
        ),
        HopOrdinal::new(hop)?,
    ))
}

fn hop(
    ordinal: u8,
    url: &str,
    started: &str,
    received: &str,
    status: u16,
    headers: HeaderMap,
) -> TestResult<ObservedHttpHopV1> {
    let request = reqwest::Request::new(reqwest::Method::GET, reqwest::Url::parse(url)?);
    let mut response = http::Response::builder()
        .status(status)
        .version(reqwest::Version::HTTP_2)
        .url(reqwest::Url::parse(url)?)
        .body(Vec::<u8>::new())?;
    *response.headers_mut() = headers;
    Ok(ObservedHttpHopV1::new(
        dispatch(ordinal)?,
        crate::net::acquisition::normalized::request(&request, started)?,
        crate::net::acquisition::normalized::response(&response.into(), received)?,
    )?)
}

pub(super) fn original() -> TestResult<(FetchOutcome, NormalizedObservedHttpV1)> {
    let body = b"<html>original source bytes\xff</html>".to_vec();
    let digest = crate::net::cache::content_digest(&body);
    let mut redirect = HeaderMap::new();
    redirect.append("location", HeaderValue::from_static("13749840"));
    let mut final_headers = HeaderMap::new();
    final_headers.append("x-source", HeaderValue::from_static("first"));
    final_headers.append("x-source", HeaderValue::from_bytes(&[0x80, 0xff])?);
    final_headers.append("content-encoding", HeaderValue::from_static("identity"));
    let observed = NormalizedObservedHttpV1::new(
        vec![
            hop(0, URL, START, "2026-10-21T08:00:01Z", 302, redirect)?,
            hop(1, FINAL, "2026-10-21T08:00:02Z", HEAD, 200, final_headers)?,
        ],
        DONE,
        AcquisitionDigest::parse(&digest)?,
        u64::try_from(body.len())?,
    )?;
    let capture = FetchOutcome {
        url: URL.into(),
        response_url: Some(FINAL.into()),
        method: "GET".into(),
        status: 200,
        content_digest: digest,
        bytes: body.len(),
        fetched_at: HEAD.into(),
        from_cache: false,
        content_type: Some("text/html".into()),
        body,
    };
    Ok((capture, observed))
}

pub(super) fn encoded(observed: &NormalizedObservedHttpV1) -> TestResult<Vec<u8>> {
    Ok(serde_json::to_vec(&json!({
        "observed_http": observed,
        "method": "GET",
        "representation": [["accept-encoding", "identity"]],
        "historical_extra": "preserved"
    }))?)
}

pub(super) fn changed(bytes: &[u8], path: &str, replacement: Value) -> TestResult<Vec<u8>> {
    let mut value: Value = serde_json::from_slice(bytes)?;
    *value.pointer_mut(path).ok_or("metadata fixture path")? = replacement;
    Ok(serde_json::to_vec(&value)?)
}
