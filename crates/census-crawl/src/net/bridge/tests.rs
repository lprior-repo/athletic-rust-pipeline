use super::{Action, BrowserError, BrowserOutcome, RequestSpec, Verdict};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const CENSUS_BIO_REQUEST: &str =
    include_str!("../../../../../fixtures/wire/athleticnet-browser-request.json");

const CAPTURE_FIXTURE: &str =
    include_str!("../../../../../fixtures/wire/athleticnet-browser-capture.json");

const FAILURE_FIXTURE: &str =
    include_str!("../../../../../fixtures/wire/athleticnet-browser-failure.json");

#[test]
fn the_census_mirror_decodes_the_fixture() -> TestResult {
    let spec: RequestSpec = serde_json::from_str(CENSUS_BIO_REQUEST)?;
    let value_of = |name: &str| -> Option<String> {
        let (_, query) = spec.url.split_once('?')?;
        query
            .split('&')
            .filter_map(|pair| pair.split_once('='))
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value.to_string())
    };
    check!(eq; value_of("level").as_deref(), Some("4"));
    check!(eq; value_of("sport").as_deref(), Some("tf"));
    check!(
        value_of("athleteId").is_some(),
        "the census asks for one named athlete, never a listing"
    );
    check!(eq;
        spec.semantic_url, spec.url,
        "the census cites the address it asked for"
    );
    check!(matches!(spec.action, Action::Fetch { body: None }));
    Ok(())
}

#[test]
fn the_census_mirror_re_encodes_the_fixture_byte_for_byte() -> TestResult {
    let spec: RequestSpec = serde_json::from_str(CENSUS_BIO_REQUEST)?;
    check!(eq;
        serde_json::to_string_pretty(&spec)?,
        CENSUS_BIO_REQUEST.trim_end(),
        "the mirror is the contract: a field added, renamed, reordered or dropped fails here"
    );
    Ok(())
}

#[test]
fn the_census_mirror_decodes_the_capture_fixture() -> TestResult {
    let outcome: BrowserOutcome = serde_json::from_str(CAPTURE_FIXTURE)?;
    let BrowserOutcome::Captured(capture) = outcome else {
        return Err("the capture fixture is a capture".into());
    };
    check!(capture.challenge, "the transport's own verdict on the page");
    check!(eq; capture.response.status, 403);
    check!(eq; capture.retry_after_ms, Some(30_000));
    check!(eq; capture.fetched_at_ms, Some(1_758_542_400_000));
    check!(capture.response.rankings.is_none());
    let cookies: Vec<&str> = capture
        .response
        .headers
        .iter()
        .filter(|(name, _)| name == "set-cookie")
        .map(|(_, value)| value.as_str())
        .collect();
    check!(eq;
        cookies,
        vec!["session=abc; Path=/", "cf_clearance=def; Path=/"],
        "a repeated header name keeps every value it carried"
    );
    check!(eq;
        capture
            .response
            .headers
            .iter()
            .find(|(name, _)| name == "retry-after")
            .map(|(_, value)| value.as_str()),
        Some("30")
    );
    let page = BASE64.decode(capture.response.body.as_bytes())?;
    let page = String::from_utf8(page)?;
    check!(
        page.contains("challenge-platform"),
        "the fixture's page is a challenge page: {page}"
    );
    Ok(())
}

#[test]
fn the_census_mirror_decodes_the_failure_fixture() -> TestResult {
    let outcome: BrowserOutcome = serde_json::from_str(FAILURE_FIXTURE)?;
    let BrowserOutcome::Failed(failure) = outcome else {
        return Err("the failure fixture is a failure".into());
    };
    check!(eq; failure.error, BrowserError::HumanRequired);
    check!(eq; failure.verdict, Verdict::HumanRequired);
    Ok(())
}

#[test]
fn every_reason_renders_as_its_wire_name() {
    const REASONS: [BrowserError; 9] = [
        BrowserError::Transport,
        BrowserError::Timeout,
        BrowserError::PayloadLimit,
        BrowserError::Redirect,
        BrowserError::Unavailable,
        BrowserError::HumanRequired,
        BrowserError::Protocol,
        BrowserError::Shutdown,
        BrowserError::TaskPanicked,
    ];
    let mut rendered: Vec<&str> = Vec::new();
    for reason in REASONS {
        let name = reason.as_str();
        assert_eq!(
            reason.to_string(),
            name,
            "the display is the wire's own spelling"
        );
        match serde_json::from_str::<BrowserError>(&format!("\"{name}\"")) {
            Ok(parsed) => assert_eq!(parsed, reason, "{name} decodes to a different reason"),
            Err(error) => panic!("{name} is not the name this decoder reads back: {error}"),
        }
        assert!(
            !rendered.contains(&name),
            "{name} is rendered for two reasons"
        );
        rendered.push(name);
    }
    assert_eq!(rendered.len(), REASONS.len());
}

#[test]
fn session_key_is_profile_zero() {
    assert_eq!(
        super::SESSION_KEY,
        "profile-0",
        "browser session must be profile-0"
    );
}

#[test]
fn admitted_browser_origins_contains_athletic_net() {
    assert!(
        super::ADMITTED_BROWSER_ORIGINS.contains(&"www.athletic.net"),
        "athletic.net must be in the admitted set"
    );
}

#[test]
fn a_non_admitted_origin_is_refused_with_policy_error() -> TestResult {
    let err = match crate::net::bridge::validate_origin("http://evil.example.com/page") {
        Err(err) => err,
        Ok(()) => return Err("non-admitted origin must be refused".into()),
    };
    match err {
        crate::net::FetchError::Policy { ref detail } => {
            check!(
                detail.contains("evil.example.com"),
                "policy error names the offending origin: {detail}"
            );
            check!(
                detail.contains("www.athletic.net"),
                "policy error names the allowed origins: {detail}"
            );
        }
        other => return Err(format!("expected Policy error, got: {other:?}").into()),
    }
    Ok(())
}

#[test]
fn an_admitted_origin_passes_validation() {
    let result = crate::net::bridge::validate_origin(
        "https://www.athletic.net/api/v1/AthleteBio/GetAthleteBioData?athleteId=1&sport=tf",
    );
    assert!(result.is_ok(), "admitted origin must pass: {result:?}");
}

#[test]
fn a_non_http_scheme_is_refused_even_when_the_host_is_admitted() -> TestResult {
    for url in [
        "file://www.athletic.net/etc/passwd",
        "ftp://www.athletic.net/mirror",
        "chrome://www.athletic.net/settings",
    ] {
        let err = match crate::net::bridge::validate_origin(url) {
            Err(err) => err,
            Ok(()) => return Err(format!("{url} must be refused before submission").into()),
        };
        match err {
            crate::net::FetchError::Policy { detail } => check!(
                detail.contains("scheme"),
                "the refusal names the scheme, not only the host: {detail}"
            ),
            other => return Err(format!("expected Policy error for {url}, got: {other:?}").into()),
        }
    }
    Ok(())
}

#[tokio::test]
async fn the_lane_refuses_a_non_http_scheme_before_any_ingress_call() -> TestResult {
    let origin = "http://127.0.0.1:1/".parse()?;
    let lane = crate::net::bridge::BrowserLane::over(crate::ingress::client(
        origin,
        reqwest::Client::new(),
    )?);
    for url in [
        "file://www.athletic.net/etc/passwd",
        "ftp://www.athletic.net/mirror",
    ] {
        let spec = RequestSpec {
            url: url.to_string(),
            semantic_url: url.to_string(),
            action: Action::Fetch { body: None },
            headers: crate::net::RepresentationHeaders::default(),
        };
        match lane.answer(&spec).await {
            Err(crate::net::FetchError::Policy { detail }) => check!(
                detail.contains("scheme"),
                "the lane's refusal names the scheme: {detail}"
            ),
            other => {
                return Err(format!(
                    "{url} must be refused by policy before any ingress call, got: {other:?}"
                )
                .into())
            }
        }
    }
    Ok(())
}
