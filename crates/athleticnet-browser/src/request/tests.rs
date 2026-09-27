use super::{RequestAction, RequestSpec};

const CENSUS_BIO_REQUEST: &str =
    include_str!("../../../../fixtures/wire/athleticnet-browser-request.json");

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

#[test]
fn re_encoding_the_fixture_is_byte_identical() {
    let spec: RequestSpec = serde_json::from_str(CENSUS_BIO_REQUEST).expect("fixture decodes");
    assert_eq!(
        serde_json::to_string_pretty(&spec).expect("re-encode"),
        CENSUS_BIO_REQUEST.trim_end(),
        "the fixture is the contract: a field added, renamed, reordered or dropped fails here"
    );
}
