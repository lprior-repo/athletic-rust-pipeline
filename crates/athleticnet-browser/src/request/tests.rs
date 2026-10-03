use super::{RequestAction, RequestSpec};

const CENSUS_BIO_REQUEST: &str =
    include_str!("../../../../fixtures/wire/athleticnet-browser-request.json");

#[test]
fn the_census_request_fixture_decodes_into_the_spec() -> Result<(), Box<dyn std::error::Error>> {
    let spec: RequestSpec = serde_json::from_str(CENSUS_BIO_REQUEST)?;
    let value_of = |name: &str| {
        spec.url
            .query_pairs()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.into_owned())
    };
    check!(eq; value_of("level").as_deref(), Some("4"));
    check!(eq; value_of("sport").as_deref(), Some("tf"));
    check!(
        value_of("athleteId").is_some(),
        "the census asks for one named athlete, never a listing"
    );
    check!(eq; spec.semantic_url,
spec.url.to_string(),
"a caller with one address cites the address it fetched");
    check!(matches!(spec.action, RequestAction::Fetch { body: None }));
    Ok(())
}
