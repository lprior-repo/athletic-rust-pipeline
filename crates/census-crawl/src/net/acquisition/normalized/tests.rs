use super::*;
use reqwest::header::{HeaderName, HeaderValue};
use reqwest::ResponseBuilderExt;

mod bounds;
mod request;
mod response;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const URL: &str = "https://mi.milesplit.com/athletes/13749840-brennan-smith";
const START: &str = "2026-10-21T08:00:00Z";
const HEAD: &str = "2026-10-21T08:00:01+00:00";

fn built_request(headers: HeaderMap) -> TestResult<reqwest::Request> {
    let mut request = reqwest::Request::new(reqwest::Method::GET, reqwest::Url::parse(URL)?);
    *request.headers_mut() = headers;
    Ok(request)
}

fn built_response(
    headers: HeaderMap,
    version: reqwest::Version,
    status: u16,
) -> TestResult<reqwest::Response> {
    let mut response = http::Response::builder()
        .status(status)
        .version(version)
        .url(reqwest::Url::parse(URL)?)
        .body(Vec::<u8>::new())?;
    *response.headers_mut() = headers;
    Ok(response.into())
}

fn duplicates() -> TestResult<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.append(
        HeaderName::from_bytes(b"X-Evidence")?,
        HeaderValue::from_bytes(b"first")?,
    );
    headers.append(
        HeaderName::from_bytes(b"X-Other")?,
        HeaderValue::from_bytes(b"other")?,
    );
    headers.append(
        HeaderName::from_bytes(b"X-Evidence")?,
        HeaderValue::from_bytes(&[0x80, 0xff, b'\t'])?,
    );
    headers.append(
        HeaderName::from_bytes(b"X-Evidence")?,
        HeaderValue::from_bytes(b"first")?,
    );
    Ok(headers)
}

#[test]
fn request_observes_api_order_duplicate_raw_values_and_supplied_start_exactly() -> TestResult {
    let native = built_request(duplicates()?)?;
    let expected = native
        .headers()
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_bytes()))
        .collect::<Vec<_>>();
    let observed = request(&native, START)?;
    let retained = observed
        .headers()
        .iter()
        .map(|header| (header.name(), header.value()))
        .collect::<Vec<_>>();
    check!(eq; retained, expected);
    check!(eq; observed.headers().len(), 4);
    let repeated = observed
        .headers()
        .iter()
        .filter(|header| header.name() == "x-evidence")
        .map(ObservedHttpHeaderV1::value)
        .collect::<Vec<_>>();
    check!(eq; repeated, vec![b"first".as_slice(), &[0x80, 0xff, b'\t'], b"first"]);
    check!(eq; observed.target_url(), native.url().as_str());
    check!(eq; observed.started_at(), START);
    let encoded = serde_json::to_vec(&observed)?;
    check!(eq; serde_json::from_slice::<ObservedHttpRequestV1>(&encoded)?, observed);
    check!(eq; native.headers().get_all("x-evidence").iter().count(), 3);
    Ok(())
}

#[test]
fn empty_api_request_headers_remain_empty_without_invented_defaults() -> TestResult {
    let native = built_request(HeaderMap::new())?;
    let observed = request(&native, START)?;
    check!(observed.headers().is_empty());
    check!(native.headers().is_empty());
    for invalid in [
        "",
        "2026-10-21",
        "2026-10-21T08:00:00-04:00",
        "2101-01-01T00:00:00Z",
    ] {
        check!(matches!(
            request(&native, invalid),
            Err(FetchError::CaptureDomain(_))
        ));
    }
    Ok(())
}
