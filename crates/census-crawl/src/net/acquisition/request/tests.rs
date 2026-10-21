use super::*;
use crate::net::execute::representation::{cache_extra, canonical_request, materialize_observed};
use crate::net::{RepresentationHeaders, DEFAULT_USER_AGENT};
use census_domain::model::CaptureDomainError;
use std::{collections::HashMap, path::Path, time::Duration};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const URL: &str = "https://mi.milesplit.com/athletes/13749840-brennan-smith";
const AGENT: &str = "Fixture/1.0";

#[test]
fn public_default_materializes_compact_bot_agent_and_identity_without_changing_legacy() -> TestResult
{
    let root = tempfile::tempdir()?;
    let fetcher = fetcher(root.path(), None)?;
    let request = fetcher.public_resource_request(URL, &FetchOptions::default())?;
    check!(request.observed_http_required());
    check!(eq; request.effective_user_agent(), "census-service/0.1");
    check!(eq; request.representation_headers(), &[
        ("accept".to_owned(), "*/*".to_owned()),
        ("accept-encoding".to_owned(), "identity".to_owned()),
        ("user-agent".to_owned(), "census-service/0.1".to_owned()),
    ]);
    let value = serde_json::to_value(&request)?;
    check!(eq; value.get("method"), Some(&serde_json::json!("GET")));
    check!(eq; value.get("body"), Some(&serde_json::json!("empty")));
    check!(eq; value.get("transport_recipe"), Some(&serde_json::json!("reqwest_normalized_observed_http_v1")));
    let encoded = serde_json::to_vec(&request)?;
    let cold: RequestRepresentationV1 = serde_json::from_slice(&encoded)?;
    check!(eq; cold, request);
    check!(eq; serde_json::to_vec(&cold)?, encoded);
    let legacy = fetcher.acquisition_request(URL, &FetchOptions::default())?;
    check!(!legacy.observed_http_required());
    check!(eq; legacy.effective_user_agent(), DEFAULT_USER_AGENT);
    check!(legacy.representation_headers().is_empty());
    let mut empty = reqwest::Request::new(reqwest::Method::GET, reqwest::Url::parse(URL)?);
    check!(empty.headers().is_empty());
    materialize_observed(&mut empty, request.effective_user_agent())?;
    observed_matches(&empty, &request)?;
    built_matches(&fetcher, &FetchOptions::default(), &request)?;
    no_transport(&fetcher)
}

#[test]
fn configured_agent_and_exact_caller_override_are_the_declared_effective_agents() -> TestResult {
    let root = tempfile::tempdir()?;
    let fetcher = fetcher(root.path(), Some(AGENT))?;
    let configured = fetcher.public_resource_request(URL, &FetchOptions::default())?;
    check!(eq; configured.effective_user_agent(), AGENT);
    check!(eq; configured.representation_headers(), &[
        ("accept".to_owned(), "*/*".to_owned()),
        ("accept-encoding".to_owned(), "identity".to_owned()), ("user-agent".to_owned(), AGENT.to_owned()),
    ]);
    let options = options(&[
        ("User-Agent", "Caller/2.0"),
        ("Accept-Encoding", "identity"),
    ]);
    let override_request = fetcher.public_resource_request(URL, &options)?;
    check!(eq; override_request.effective_user_agent(), "Caller/2.0");
    check!(eq; override_request.representation_headers(), &[
        ("accept".to_owned(), "*/*".to_owned()),
        ("accept-encoding".to_owned(), "identity".to_owned()), ("user-agent".to_owned(), "Caller/2.0".to_owned()),
    ]);
    check!(eq; options.headers, vec![
        ("User-Agent".to_owned(), "Caller/2.0".to_owned()),
        ("Accept-Encoding".to_owned(), "identity".to_owned()),
    ]);
    built_matches(&fetcher, &options, &override_request)?;
    no_transport(&fetcher)
}

#[test]
fn public_sorted_headers_and_cache_identity_match_the_actual_normalized_options() -> TestResult {
    let root = tempfile::tempdir()?;
    let fetcher = fetcher(root.path(), Some(AGENT))?;
    let mut options = options(&[("Accept-Language", "en-US"), ("ACCEPT", "application/json")]);
    options.refresh = true;
    options.allow_not_found = true;
    let original = options.clone();
    let request = fetcher.public_resource_request(URL, &options)?;
    let normalized = fetcher.observed_options(&options)?;
    let actual_cache = canonical_request(URL, &normalized.headers)?;
    check!(eq; request.representation_headers(), actual_cache.entries());
    check!(eq; request.representation_headers(), &[
        ("accept".to_owned(), "application/json".to_owned()),
        ("accept-encoding".to_owned(), "identity".to_owned()),
        ("accept-language".to_owned(), "en-US".to_owned()), ("user-agent".to_owned(), AGENT.to_owned()),
    ]);
    let identity = concat!(
        "accept=application/json\u{1f}accept-encoding=identity\u{1f}",
        "accept-language=en-US\u{1f}user-agent=Fixture/1.0\u{1f}"
    );
    check!(eq; cache_extra(None, &actual_cache), identity);
    check!(eq; Fetcher::key_for("GET", URL, identity), "e6e43c832a4bb36805ea22b97c2be658");
    let declared = RepresentationHeaders::from_entries(request.representation_headers().to_vec())?;
    check!(eq; declared, actual_cache);
    check!(eq; normalized.refresh, original.refresh);
    check!(eq; normalized.allow_not_found, original.allow_not_found);
    check!(eq; options.headers, original.headers);
    check!(eq; options.refresh, original.refresh);
    check!(eq; options.allow_not_found, original.allow_not_found);
    built_matches(&fetcher, &options, &request)?;
    no_transport(&fetcher)
}

#[test]
fn public_requests_refuse_credentials_encoded_bodies_and_duplicate_representation_fields(
) -> TestResult {
    let root = tempfile::tempdir()?;
    let fetcher = fetcher(root.path(), Some(AGENT))?;
    for headers in [
        vec![("Cookie", "session=controlled")],
        vec![("Authorization", "Bearer controlled")],
        vec![("Proxy-Authorization", "Basic controlled")],
        vec![("Accept-Encoding", "gzip")],
        vec![("accept-encoding", "identity ")],
        vec![("Accept", "text/html"), (" accept ", "text/html")],
        vec![("User-Agent", AGENT), ("user-agent", AGENT)],
        vec![
            ("Accept-Encoding", "identity"),
            ("accept-encoding", "identity"),
        ],
    ] {
        check!(matches!(
            fetcher.public_resource_request(URL, &options(&headers)),
            Err(FetchError::Policy { .. })
        ));
    }
    check!(matches!(
        fetcher.public_resource_request(URL, &options(&[("anettokens", "controlled")])),
        Err(FetchError::CaptureDomain(CaptureDomainError::Invalid {
            field: "observed request representation"
        }))
    ));
    no_transport(&fetcher)
}

#[test]
fn malformed_agents_refuse_and_custom_comment_form_needs_a_valid_caller_override() -> TestResult {
    let root = tempfile::tempdir()?;
    let fetcher = fetcher(root.path(), Some("Custom/1.0 (comment)"))?;
    check!(matches!(
        fetcher.public_resource_request(URL, &FetchOptions::default()),
        Err(FetchError::Policy { .. })
    ));
    let recovered = fetcher.public_resource_request(URL, &options(&[("User-Agent", AGENT)]))?;
    check!(eq; recovered.effective_user_agent(), AGENT);
    let oversized = "x".repeat(201);
    for value in [
        "",
        " ",
        " Leading/1.0",
        "Trailing/1.0 ",
        "Custom/1.0 (comment)",
        "Custom/1.0\r\nAccept: text/html",
        "Custom/1.0\u{7f}",
        "M\u{f6}bius/1.0",
        oversized.as_str(),
    ] {
        check!(matches!(
            fetcher.public_resource_request(URL, &options(&[("User-Agent", value)])),
            Err(FetchError::Policy { .. })
        ));
    }
    no_transport(&fetcher)
}

#[test]
fn conditional_headers_stay_in_options_without_changing_representation_or_cache_identity(
) -> TestResult {
    let root = tempfile::tempdir()?;
    let fetcher = fetcher(root.path(), Some(AGENT))?;
    let baseline = fetcher.public_resource_request(URL, &FetchOptions::default())?;
    let options = options(&[
        ("If-None-Match", "\"original-etag\""),
        ("If-Modified-Since", "Wed, 21 Oct 2026 08:00:00 GMT"),
    ]);
    let held = options.headers.clone();
    let request = fetcher.public_resource_request(URL, &options)?;
    check!(eq; request, baseline);
    let normalized = fetcher.observed_options(&options)?;
    let conditional = normalized
        .headers
        .iter()
        .filter(|(name, _)| {
            name.eq_ignore_ascii_case("if-none-match")
                || name.eq_ignore_ascii_case("if-modified-since")
        })
        .cloned()
        .collect::<Vec<_>>();
    check!(eq; conditional, held);
    check!(eq; options.headers, held);
    let twice = fetcher.observed_options(&normalized)?;
    check!(eq; twice.headers, normalized.headers);
    let cache = canonical_request(URL, &normalized.headers)?;
    check!(eq; cache.entries(), baseline.representation_headers());
    check!(eq; cache_extra(None, &cache), "accept=*/*\u{1f}accept-encoding=identity\u{1f}user-agent=Fixture/1.0\u{1f}");
    no_transport(&fetcher)
}

#[test]
fn legacy_request_retains_exact_original_recipe_bytes_and_refuses_identity_encoding() -> TestResult
{
    let root = tempfile::tempdir()?;
    let fetcher = fetcher(root.path(), Some(AGENT))?;
    let request = fetcher.acquisition_request(URL, &options(&[("Accept", "application/json")]))?;
    let original = concat!(
        "{\"method\":\"GET\",\"original_url\":\"https://mi.milesplit.com/athletes/13749840-brennan-smith\",",
        "\"body\":\"empty\",\"representation_headers\":[[\"accept\",\"application/json\"]],",
        "\"transport_recipe\":\"direct_http_one_shot_v1\",\"effective_user_agent\":\"Fixture/1.0\"}"
    );
    check!(eq; serde_json::to_vec(&request)?, original.as_bytes());
    let cold: RequestRepresentationV1 = serde_json::from_slice(original.as_bytes())?;
    check!(eq; cold, request);
    check!(!request.observed_http_required());
    check!(matches!(
        fetcher.acquisition_request(URL, &options(&[("Accept-Encoding", "identity")])),
        Err(FetchError::CaptureDomain(CaptureDomainError::Invalid {
            field: "representation headers"
        }))
    ));
    no_transport(&fetcher)
}

fn fetcher(root: &Path, agent: Option<&str>) -> TestResult<Fetcher> {
    Ok(Fetcher::new(
        root.join("http"),
        agent.map(str::to_owned),
        Duration::ZERO,
        HashMap::new(),
        Vec::new(),
    )?
    .with_offline(true))
}

fn options(headers: &[(&str, &str)]) -> FetchOptions {
    FetchOptions {
        headers: headers
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect(),
        ..FetchOptions::default()
    }
}

fn no_transport(fetcher: &Fetcher) -> TestResult {
    check!(eq; fetcher.stats.try_lock()?.physical_requests(), 0);
    check!(std::fs::read_dir(fetcher.cache_dir())?.next().is_none());
    Ok(())
}

fn built_matches(
    fetcher: &Fetcher,
    options: &FetchOptions,
    declared: &RequestRepresentationV1,
) -> TestResult {
    let normalized = fetcher.observed_options(options)?;
    let mut native = crate::net::request::build_request(
        &fetcher.client,
        "GET",
        URL,
        None,
        &normalized.headers,
        None,
        options.refresh,
    )?
    .build()?;
    let before = native.headers().clone();
    materialize_observed(&mut native, &fetcher.effective_user_agent)?;
    check!(eq; native.headers(), &before);
    observed_matches(&native, declared)
}

fn observed_matches(native: &reqwest::Request, declared: &RequestRepresentationV1) -> TestResult {
    let observed = super::super::normalized::request(native, "2026-10-21T08:00:00Z")?;
    check!(eq; observed.target_url(), declared.original_url());
    check!(eq; observed.headers().len(), declared.representation_headers().len());
    let mut actual = observed
        .headers()
        .iter()
        .map(|header| (header.name().to_owned(), header.value().to_vec()))
        .collect::<Vec<_>>();
    actual.sort();
    let expected = declared
        .representation_headers()
        .iter()
        .map(|(name, value)| (name.clone(), value.as_bytes().to_vec()))
        .collect::<Vec<_>>();
    check!(eq; actual, expected);
    Ok(())
}
