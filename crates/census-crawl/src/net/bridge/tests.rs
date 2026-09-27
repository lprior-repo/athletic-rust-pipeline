
use super::{Action, BrowserError, BrowserOutcome, RequestSpec, Verdict};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;

const CENSUS_BIO_REQUEST: &str =
    include_str!("../../../../../fixtures/wire/athleticnet-browser-request.json");

const CAPTURE_FIXTURE: &str =
    include_str!("../../../../../fixtures/wire/athleticnet-browser-capture.json");

const FAILURE_FIXTURE: &str =
    include_str!("../../../../../fixtures/wire/athleticnet-browser-failure.json");

#[test]
fn the_census_mirror_decodes_the_fixture() {
    let spec: RequestSpec = serde_json::from_str(CENSUS_BIO_REQUEST).expect("fixture decodes");
    let value_of = |name: &str| -> Option<String> {
        let (_, query) = spec.url.split_once('?')?;
        query
            .split('&')
            .filter_map(|pair| pair.split_once('='))
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value.to_string())
    };
    assert_eq!(value_of("level").as_deref(), Some("4"));
    assert_eq!(value_of("sport").as_deref(), Some("tf"));
    assert!(
        value_of("athleteId").is_some(),
        "the census asks for one named athlete, never a listing"
    );
    assert_eq!(
        spec.semantic_url, spec.url,
        "the census cites the address it asked for"
    );
    assert!(matches!(spec.action, Action::Fetch { body: None }));
}

#[test]
fn the_census_mirror_re_encodes_the_fixture_byte_for_byte() {
    let spec: RequestSpec = serde_json::from_str(CENSUS_BIO_REQUEST).expect("fixture decodes");
    assert_eq!(
        serde_json::to_string_pretty(&spec).expect("re-encode"),
        CENSUS_BIO_REQUEST.trim_end(),
        "the mirror is the contract: a field added, renamed, reordered or dropped fails here"
    );
}

#[test]
fn the_census_mirror_decodes_the_capture_fixture() {
    let outcome: BrowserOutcome = serde_json::from_str(CAPTURE_FIXTURE).expect("fixture decodes");
    let BrowserOutcome::Captured(capture) = outcome else {
        panic!("the capture fixture is a capture");
    };
    assert!(capture.challenge, "the transport's own verdict on the page");
    assert_eq!(capture.response.status, 403);
    assert_eq!(capture.retry_after_ms, Some(30_000));
    assert_eq!(capture.fetched_at_ms, Some(1_758_542_400_000));
    assert!(capture.response.rankings.is_none());
    let cookies: Vec<&str> = capture
        .response
        .headers
        .iter()
        .filter(|(name, _)| name == "set-cookie")
        .map(|(_, value)| value.as_str())
        .collect();
    assert_eq!(
        cookies,
        vec!["session=abc; Path=/", "cf_clearance=def; Path=/"],
        "a repeated header name keeps every value it carried"
    );
    assert_eq!(
        capture
            .response
            .headers
            .iter()
            .find(|(name, _)| name == "retry-after")
            .map(|(_, value)| value.as_str()),
        Some("30")
    );
    let page = BASE64
        .decode(capture.response.body.as_bytes())
        .expect("the body arrives base64");
    let page = String::from_utf8(page).expect("an HTML challenge page is UTF-8");
    assert!(
        page.contains("challenge-platform"),
        "the fixture's page is a challenge page: {page}"
    );
}

#[test]
fn the_census_mirror_decodes_the_failure_fixture() {
    let outcome: BrowserOutcome = serde_json::from_str(FAILURE_FIXTURE).expect("fixture decodes");
    let BrowserOutcome::Failed(failure) = outcome else {
        panic!("the failure fixture is a failure");
    };
    assert_eq!(failure.error, BrowserError::HumanRequired);
    assert_eq!(failure.verdict, Verdict::HumanRequired);
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
fn a_non_admitted_origin_is_refused_with_policy_error() {
    let err = crate::net::bridge::validate_origin("http://evil.example.com/page")
        .expect_err("non-admitted origin must be refused");
    match err {
        crate::net::FetchError::Policy { ref detail } => {
            assert!(
                detail.contains("evil.example.com"),
                "policy error names the offending origin: {detail}"
            );
            assert!(
                detail.contains("www.athletic.net"),
                "policy error names the allowed origins: {detail}"
            );
        }
        other => panic!("expected Policy error, got: {other:?}"),
    }
}

#[test]
fn an_admitted_origin_passes_validation() {
    let result = crate::net::bridge::validate_origin(
        "https://www.athletic.net/api/v1/AthleteBio/GetAthleteBioData?athleteId=1&sport=tf",
    );
    assert!(result.is_ok(), "admitted origin must pass: {result:?}");
}
