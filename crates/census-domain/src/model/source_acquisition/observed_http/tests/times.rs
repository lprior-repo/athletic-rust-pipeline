use super::*;

#[test]
fn timestamps_require_rfc3339_zero_offset_and_year_range_without_rewriting() -> TestResult {
    for value in [
        "1900-01-01T00:00:00Z",
        "2100-12-31T23:59:59+00:00",
        "2026-10-21T08:00:00.123456789+00:00",
    ] {
        let request = ObservedHttpRequestV1::new(fixture::URL, value, Vec::new())?;
        check!(eq; request.started_at(), value);
        check!(eq; serde_json::from_slice::<ObservedHttpRequestV1>(&serde_json::to_vec(&request)?)?, request);
    }
    for value in [
        "1899-12-31T23:59:59Z",
        "2101-01-01T00:00:00Z",
        "2026-10-21",
        "2026-10-21T08:00:00+01:00",
        "2026-10-21T08:00:00-05:00",
        "not-a-time",
    ] {
        check!(ObservedHttpRequestV1::new(fixture::URL, value, Vec::new()).is_err());
        check!(ObservedHttpResponseV1::new(
            ObservedHttpProtocolV1::Http2,
            200,
            fixture::URL,
            value,
            Vec::new()
        )
        .is_err());
    }
    Ok(())
}

#[test]
fn hop_start_cannot_follow_response_head_and_completion_cannot_precede_it() -> TestResult {
    let request = ObservedHttpRequestV1::new(fixture::URL, fixture::DONE, Vec::new())?;
    check!(ObservedHttpHopV1::new(
        fixture::dispatch(0, 1)?,
        request,
        fixture::response(Vec::new())?
    )
    .is_err());
    let original = fixture::original()?;
    for value in ["2026-10-21T07:59:59Z", "2026-10-21T08:00:00.999Z"] {
        check!(NormalizedObservedHttpV1::new(
            original.hops().to_vec(),
            value,
            fixture::digest()?,
            4
        )
        .is_err());
    }
    check!(NormalizedObservedHttpV1::new(
        original.hops().to_vec(),
        "2026-10-21T08:00:01+00:00",
        fixture::digest()?,
        4
    )
    .is_ok());
    Ok(())
}

#[test]
fn contiguous_hop_clock_order_compares_instants_including_fractional_seconds() -> TestResult {
    let original = fixture::observed(fixture::chain(2)?)?;
    let mut value = serde_json::to_value(&original)?;
    value["hops"][1]["request"]["started_at"] = serde_json::json!("2026-10-21T08:00:00.999999999Z");
    check!(serde_json::from_value::<NormalizedObservedHttpV1>(value.clone()).is_err());
    value["hops"][1]["request"]["started_at"] = serde_json::json!("2026-10-21T08:00:01+00:00");
    let equal: NormalizedObservedHttpV1 = serde_json::from_value(value)?;
    check!(eq; equal.hops()[1].request().started_at(), "2026-10-21T08:00:01+00:00");
    Ok(())
}
