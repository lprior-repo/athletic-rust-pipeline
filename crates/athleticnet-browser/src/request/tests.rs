//! The cross-crate wire contract, pinned on the side that owns the shape.
//!
//! The census acquires Athletic.net through this transport but may not depend on this crate: linking
//! it would put the browser engine, and with it the ability to open a second manager on the one
//! profile, inside the census's own dependency tree. So the census mirrors [`RequestSpec`] and the
//! two shapes are held together by one committed fixture per direction under `fixtures/wire/`, which
//! both crates' tests read. This side asserts the fixture is byte-for-byte what these types encode,
//! so a field added, renamed, reordered or dropped here fails there.
//!
//! The fixture is the census's own request: its endpoint parameters and its `level`, which the
//! pipeline's request builder never produces (`build.rs` hardcodes `level=0`).

use super::{RequestAction, RequestSpec};

/// One athlete-bio request, as the census builds it.
const CENSUS_BIO_REQUEST: &str =
    include_str!("../../../../fixtures/wire/athleticnet-browser-request.json");

/// The fixture decodes into this crate's own type, and says what it claims to say.
#[test]
fn the_census_request_fixture_decodes_into_the_spec() {
    let spec: RequestSpec = serde_json::from_str(CENSUS_BIO_REQUEST).expect("fixture decodes");
    let value_of = |name: &str| {
        spec.url
            .query_pairs()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.into_owned())
    };
    assert_eq!(value_of("level").as_deref(), Some("4"));
    assert_eq!(value_of("sport").as_deref(), Some("tf"));
    assert!(
        value_of("athleteId").is_some(),
        "the census asks for one named athlete, never a listing"
    );
    assert_eq!(
        spec.semantic_url,
        spec.url.to_string(),
        "a caller with one address cites the address it fetched"
    );
    assert!(matches!(spec.action, RequestAction::Fetch { body: None }));
}

/// Re-encoding the fixture reproduces it byte for byte.
#[test]
fn re_encoding_the_fixture_is_byte_identical() {
    let spec: RequestSpec = serde_json::from_str(CENSUS_BIO_REQUEST).expect("fixture decodes");
    assert_eq!(
        serde_json::to_string_pretty(&spec).expect("re-encode"),
        CENSUS_BIO_REQUEST.trim_end(),
        "the fixture is the contract: a field added, renamed, reordered or dropped fails here"
    );
}
