//! The census's half of the browser-lane wire contract.
//!
//! The fixtures are shared with `athleticnet-browser`; that crate's own tests assert its types encode
//! them byte for byte, and these assert the mirror reads — and, for the request, re-encodes — the same
//! bytes. A change to either shape fails on the other side.

use super::{Action, BrowserError, BrowserOutcome, RequestSpec, Verdict};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;

/// One athlete-bio request, as the census builds it.
const CENSUS_BIO_REQUEST: &str =
    include_str!("../../../../../fixtures/wire/athleticnet-browser-request.json");

/// The transport's own answer for a request that met a challenge.
const CAPTURE_FIXTURE: &str =
    include_str!("../../../../../fixtures/wire/athleticnet-browser-capture.json");

/// The transport's own answer for a request it stopped without capturing.
const FAILURE_FIXTURE: &str =
    include_str!("../../../../../fixtures/wire/athleticnet-browser-failure.json");

/// The census's own reading of the fixture, field by field.
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

/// The census re-encodes the fixture byte for byte, so what it posts is what the crate reads.
#[test]
fn the_census_mirror_re_encodes_the_fixture_byte_for_byte() {
    let spec: RequestSpec = serde_json::from_str(CENSUS_BIO_REQUEST).expect("fixture decodes");
    assert_eq!(
        serde_json::to_string_pretty(&spec).expect("re-encode"),
        CENSUS_BIO_REQUEST.trim_end(),
        "the mirror is the contract: a field added, renamed, reordered or dropped fails here"
    );
}

/// The transport's own challenge capture, decoded field by field.
///
/// The status, the repeated `set-cookie` name and the base64 body are the three readings the request
/// direction never exercises, so this is where the answer half of the mirror is proved: a challenge
/// capture must arrive with its flag, its clock and its own `Retry-After`, and the page it carries
/// must be the verification page — which is what makes refusing it, rather than caching it, correct.
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

/// The transport's own failure answer, decoded into the two readings that travel with it.
#[test]
fn the_census_mirror_decodes_the_failure_fixture() {
    let outcome: BrowserOutcome = serde_json::from_str(FAILURE_FIXTURE).expect("fixture decodes");
    let BrowserOutcome::Failed(failure) = outcome else {
        panic!("the failure fixture is a failure");
    };
    assert_eq!(failure.error, BrowserError::HumanRequired);
    assert_eq!(failure.verdict, Verdict::HumanRequired);
}

/// Every reason renders as the name the wire gives it.
///
/// The census refuses a lane failure with the reason in the message, so the renderer and the serde
/// name have to be one spelling: this reads each rendered name back through the decoder, which is
/// what makes a message an operator reads a property of the contract rather than of this file.
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
