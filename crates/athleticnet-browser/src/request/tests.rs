use super::{RequestAction, RequestSpec};

const CENSUS_BIO_REQUEST: &str =
    include_str!("../../../../fixtures/wire/athleticnet-browser-request.json");

fn current_profile_request() -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let mut request: serde_json::Value = serde_json::from_str(CENSUS_BIO_REQUEST)?;
    let action = request
        .get_mut("action")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or("captured request action")?;
    check!(eq; action.remove("body"), None);
    Ok(request)
}

#[test]
fn the_profile_request_preserves_the_captured_url() -> Result<(), Box<dyn std::error::Error>> {
    let spec: RequestSpec = serde_json::from_value(current_profile_request()?)?;
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
    check!(matches!(spec.action, RequestAction::Fetch {}));
    check!(eq; spec.body(), None);
    check!(eq; serde_json::to_value(&spec.action)?, serde_json::json!({"action": "fetch"}));
    Ok(())
}

#[test]
fn obsolete_search_body_is_rejected_before_dispatch() -> Result<(), Box<dyn std::error::Error>> {
    for body in [
        serde_json::json!({"q": "*", "fq": "type:athlete", "start": 0}),
        serde_json::Value::Null,
    ] {
        let mut request: serde_json::Value = serde_json::from_str(CENSUS_BIO_REQUEST)?;
        let action = request
            .get_mut("action")
            .and_then(serde_json::Value::as_object_mut)
            .ok_or("captured request action")?;
        action.insert("body".into(), body);
        let decoded = serde_json::from_value::<RequestSpec>(request);
        let error = decoded.err().ok_or("obsolete search body was accepted")?;
        check!(eq; error.classify(), serde_json::error::Category::Data);
        check!(error.to_string().contains("body"));
    }
    Ok(())
}

#[test]
fn same_origin_holds_scheme_host_and_port_together() -> Result<(), Box<dyn std::error::Error>> {
    let origin = url::Url::parse("https://www.athletic.net/")?;
    let verdict = |value: &str| -> Result<bool, Box<dyn std::error::Error>> {
        Ok(super::same_origin(&url::Url::parse(value)?, &origin))
    };
    check!(
        verdict("https://www.athletic.net/api/v1/AthleteBio/GetAthleteBioData?athleteId=1")?,
        "the source's own https pages are the configured origin"
    );
    check!(
        verdict("https://www.athletic.net:443/api/v1/x")?,
        "an explicit default port is still the configured origin"
    );
    check!(
        !verdict("http://www.athletic.net/api/v1/x")?,
        "plain http escapes an https profile"
    );
    check!(
        !verdict("file://www.athletic.net/etc/passwd")?,
        "a file URL is never the profile origin"
    );
    check!(
        !verdict("chrome://www.athletic.net/settings")?,
        "a browser-internal scheme is never the profile origin"
    );
    check!(
        !verdict("https://www.athletic.net.evil.example/api/v1/x")?,
        "a host that merely ends in the source name is foreign"
    );
    check!(
        !verdict("https://www.athletic.net:8443/x")?,
        "an unconfigured port is foreign"
    );
    check!(
        !verdict("https://evil.example/api/v1/x")?,
        "another host is foreign"
    );
    Ok(())
}
