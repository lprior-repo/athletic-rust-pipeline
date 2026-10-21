use super::*;

fn check_refused(headers: HeaderMap, field: &'static str, max: usize) -> TestResult {
    let native = built_request(headers.clone())?;
    check!(
        matches!(request(&native, START), Err(FetchError::CaptureDomain(CaptureDomainError::Bound { field: actual, max: actual_max })) if actual == field && actual_max == max)
    );
    let native = built_response(headers, reqwest::Version::HTTP_11, 200)?;
    check!(
        matches!(super::super::response(&native, HEAD), Err(FetchError::CaptureDomain(CaptureDomainError::Bound { field: actual, max: actual_max })) if actual == field && actual_max == max)
    );
    Ok(())
}

#[test]
fn occurrence_bound_counts_every_duplicate_rather_than_unique_names() -> TestResult {
    let mut headers = HeaderMap::new();
    for _ in 0..32 {
        headers.append(
            HeaderName::from_bytes(b"x-repeat")?,
            HeaderValue::from_bytes(b"same")?,
        );
    }
    check!(eq; headers.keys_len(), 1);
    check!(eq; headers.len(), 32);
    let native = built_request(headers.clone())?;
    check!(eq; request(&native, START)?.headers().len(), 32);
    let native = built_response(headers.clone(), reqwest::Version::HTTP_11, 200)?;
    check!(eq; super::super::response(&native, HEAD)?.headers().len(), 32);
    headers.append(
        HeaderName::from_bytes(b"x-repeat")?,
        HeaderValue::from_bytes(b"same")?,
    );
    check_refused(headers, "observed header occurrences", 32)
}

#[test]
fn exact_header_name_and_raw_value_limits_admit_then_one_more_refuses() -> TestResult {
    let name = "x".repeat(128);
    let bytes = vec![0xff; 2048];
    let mut headers = HeaderMap::new();
    headers.append(
        HeaderName::from_bytes(name.as_bytes())?,
        HeaderValue::from_bytes(&bytes)?,
    );
    let native = built_request(headers.clone())?;
    let observed = request(&native, START)?;
    let header = observed
        .headers()
        .first()
        .ok_or("observed boundary header")?;
    check!(eq; header.name(), name);
    check!(eq; header.value(), bytes);
    let native = built_response(headers, reqwest::Version::HTTP_11, 200)?;
    check!(eq; super::super::response(&native, HEAD)?.headers(), observed.headers());
    let mut oversized_name = HeaderMap::new();
    oversized_name.append(
        HeaderName::from_bytes("x".repeat(129).as_bytes())?,
        HeaderValue::from_bytes(b"value")?,
    );
    check_refused(oversized_name, "observed header name", 128)?;
    let mut oversized_value = HeaderMap::new();
    oversized_value.append(
        HeaderName::from_bytes(b"x-last")?,
        HeaderValue::from_bytes(&vec![0xff; 2049])?,
    );
    check_refused(oversized_value, "observed header value", 2048)
}
