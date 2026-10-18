use super::policy::{capture_is_current, Policy};
use crate::net::{FetchError, FetchOutcome};
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
const ROOT: &str = "https://school.example/";

fn capture(body: &[u8], status: u16) -> FetchOutcome {
    FetchOutcome {
        url: "https://school.example/robots.txt".to_owned(),
        response_url: None,
        method: "GET".to_owned(),
        status,
        content_digest: crate::net::cache::content_digest(body),
        bytes: body.len(),
        fetched_at: "2026-10-18T00:00:00Z".to_owned(),
        from_cache: true,
        content_type: Some("text/plain".to_owned()),
        body: body.to_vec(),
    }
}

#[test]
fn policy_selects_actual_product_and_merges_groups_with_allow_ties() -> TestResult {
    let body = b"User-agent: *\nDisallow: /\nUser-agent: Census-Service\nDisallow: /private\nAllow: /private/public\nUser-agent: census-service\nDisallow: /second\nDisallow: /tie\nAllow: /tie\n";
    let policy = Policy::from_capture(
        &url::Url::parse(ROOT)?,
        &capture(body, 200),
        "census-service/0.1",
    )?;
    for path in ["/", "/private/public", "/tie"] {
        policy.check(&format!("https://school.example{path}"))?;
    }
    for path in ["/private", "/second"] {
        check!(matches!(
            policy.check(&format!("https://school.example{path}")),
            Err(FetchError::Policy { .. })
        ));
    }
    Ok(())
}

#[test]
fn wildcard_end_anchor_and_query_are_checked_without_fragment() -> TestResult {
    let policy = Policy::from_capture(
        &url::Url::parse(ROOT)?,
        &capture(b"User-agent: *\nDisallow: /*?s=coach$\n", 200),
        "census-service/0.1",
    )?;
    check!(policy
        .check("https://school.example/?s=coach#anchor")
        .is_err());
    policy.check("https://school.example/?s=coaching")?;
    Ok(())
}

#[test]
fn missing_policy_permits_but_forbidden_and_unsupported_policy_refuse() -> TestResult {
    let target = url::Url::parse(ROOT)?;
    Policy::from_capture(&target, &capture(b"missing", 404), "test/1")?.check(ROOT)?;
    for status in [401, 403, 429, 500, 503] {
        check!(Policy::from_capture(&target, &capture(b"", status), "test/1").is_err());
    }
    for body in [
        b"<html>challenge</html>".as_slice(),
        b"User-agent: *\nDisallow: /%73taff\n",
        b"User-agent: *\nCrawl-delay: 2\nCrawl-delay: 1\n",
        b"User-agent: *\nCrawl-delay: NaN\n",
        b"User-agent: *\nCrawl-delay: -1\n",
        b"User-agent: *\nCrawl-delay: 61\n",
        b"User-agent: *\nDisallow: /a$b\n",
        b"\xff",
    ] {
        check!(Policy::from_capture(&target, &capture(body, 200), "test/1").is_err());
    }
    Ok(())
}

#[test]
fn parser_budgets_refuse_without_truncating_rules() -> TestResult {
    let target = url::Url::parse(ROOT)?;
    for body in [
        vec![b' '; 500 * 1024 + 1],
        "# comment\n".repeat(513).into_bytes(),
        format!("User-agent: *\nDisallow: /{}", "x".repeat(2048)).into_bytes(),
    ] {
        check!(Policy::from_capture(&target, &capture(&body, 200), "test/1").is_err());
    }
    Ok(())
}

#[test]
fn policy_origin_includes_scheme_and_port_and_refuses_encoded_targets() -> TestResult {
    let policy = Policy::from_capture(&url::Url::parse(ROOT)?, &capture(b"", 404), "test/1")?;
    for url in [
        "http://school.example/",
        "https://school.example:8443/",
        "https://other.example/",
        "https://school.example/%73taff",
    ] {
        check!(policy.check(url).is_err());
    }
    Ok(())
}

#[test]
fn crawl_delay_zero_is_supported_and_retrieval_time_does_not_advance() -> TestResult {
    let policy = Policy::from_capture(
        &url::Url::parse(ROOT)?,
        &capture(b"User-agent: *\nCrawl-delay: 0\n", 200),
        "test/1",
    )?;
    check!(eq; policy.delay(), Duration::ZERO);
    let evidence = capture(b"", 404);
    for (now, expected) in [
        ("2026-10-18T12:00:00Z", true),
        ("2026-10-19T00:00:01Z", false),
        ("2026-10-17T23:59:59Z", false),
    ] {
        let now = chrono::DateTime::parse_from_rfc3339(now)?.with_timezone(&chrono::Utc);
        check!(eq; capture_is_current(&evidence, now)?, expected);
    }
    check!(eq; evidence.fetched_at, "2026-10-18T00:00:00Z");
    Ok(())
}

#[path = "transport_tests.rs"]
mod transport;

#[test]
fn empty_disallow_and_comment_only_policies_allow_public_paths() -> TestResult {
    for body in [
        b"User-agent: *\nDisallow:\n".as_slice(),
        b"# no exclusions\n",
        b"",
    ] {
        Policy::from_capture(
            &url::Url::parse(ROOT)?,
            &capture(body, 200),
            "census-service/0.1",
        )?
        .check("https://school.example/staff")?;
    }
    Ok(())
}
