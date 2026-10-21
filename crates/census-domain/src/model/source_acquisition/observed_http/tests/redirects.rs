use super::*;

fn relative_chain() -> TestResult<NormalizedObservedHttpV1> {
    let first = fixture::redirect(fixture::hop(0, Vec::new(), Vec::new())?, "./next?view=raw")?;
    let target = "https://example.test/next?view=raw";
    let second = fixture::hop(1, Vec::new(), Vec::new())?;
    let next = ObservedHttpHopV1::new(
        second.dispatch().clone(),
        ObservedHttpRequestV1::new(target, second.request().started_at(), Vec::new())?,
        ObservedHttpResponseV1::new(
            ObservedHttpProtocolV1::Http11,
            200,
            target,
            second.response().received_at(),
            Vec::new(),
        )?,
    )?;
    fixture::observed(vec![first, next])
}

#[test]
fn relative_location_binds_next_target_and_preserves_actual_headers_and_protocols() -> TestResult {
    let observed = relative_chain()?;
    let raw = serde_json::to_vec(&observed)?;
    let cold: NormalizedObservedHttpV1 = serde_json::from_slice(&raw)?;
    check!(eq; cold, observed);
    check!(eq; cold.hops()[0].response().headers()[0].value(), b"./next?view=raw");
    check!(eq; cold.hops()[1].request().target_url(), "https://example.test/next?view=raw");
    check!(eq; cold.hops()[0].response().protocol(), ObservedHttpProtocolV1::Http2);
    check!(eq; cold.hops()[1].response().protocol(), ObservedHttpProtocolV1::Http11);
    Ok(())
}

#[test]
fn only_closed_redirect_statuses_allow_an_intermediate_hop() -> TestResult {
    let original = serde_json::to_value(relative_chain()?)?;
    for status in [301, 302, 303, 307, 308] {
        let mut value = original.clone();
        value["hops"][0]["response"]["status"] = serde_json::json!(status);
        check!(serde_json::from_value::<NormalizedObservedHttpV1>(value).is_ok());
    }
    for status in [200, 300, 304, 305, 306, 399, 400, 500] {
        let mut value = original.clone();
        value["hops"][0]["response"]["status"] = serde_json::json!(status);
        check!(serde_json::from_value::<NormalizedObservedHttpV1>(value).is_err());
    }
    Ok(())
}

#[test]
fn missing_duplicate_invalid_control_and_wrong_target_locations_refuse() -> TestResult {
    let original = serde_json::to_value(relative_chain()?)?;
    let valid = ObservedHttpHeaderV1::new("location", b"./next?view=raw".to_vec())?;
    let mut alternatives = vec![Vec::new(), vec![valid.clone(), valid]];
    for value in [
        b"./wrong?view=raw".as_slice(),
        b"http://[",
        b"\t./next?view=raw",
        b"",
        &[0xff],
    ] {
        alternatives.push(vec![ObservedHttpHeaderV1::new("location", value.to_vec())?]);
    }
    for headers in alternatives {
        let mut value = original.clone();
        value["hops"][0]["response"]["headers"] = serde_json::to_value(headers)?;
        check!(serde_json::from_value::<NormalizedObservedHttpV1>(value).is_err());
    }
    let mut wrong_next = original;
    wrong_next["hops"][1]["request"]["target_url"] = serde_json::json!(fixture::URL);
    wrong_next["hops"][1]["response"]["response_url"] = serde_json::json!(fixture::URL);
    check!(serde_json::from_value::<NormalizedObservedHttpV1>(wrong_next).is_err());
    Ok(())
}

#[test]
fn terminal_redirect_is_an_observation_but_terminal_304_cannot_relabel_original_body() -> TestResult
{
    let original = serde_json::to_value(fixture::original()?)?;
    for status in [301, 302, 303, 307, 308] {
        let mut value = original.clone();
        value["hops"][0]["response"]["status"] = serde_json::json!(status);
        let observed: NormalizedObservedHttpV1 = serde_json::from_value(value)?;
        check!(eq; observed.hops()[0].response().status(), status);
        check!(observed.hops()[0].response().headers().is_empty());
    }
    let mut revalidation = original;
    revalidation["hops"][0]["response"]["status"] = serde_json::json!(304);
    check!(serde_json::from_value::<NormalizedObservedHttpV1>(revalidation).is_err());
    Ok(())
}
