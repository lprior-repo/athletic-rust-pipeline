use super::{fixture as f, *};

#[test]
fn observed_response_requires_request_head_and_preserves_exact_raw_bytes_on_cold_read() -> TestResult
{
    let work = f::public()?;
    let dispatch = f::dispatch(&work, 0)?;
    let request = f::request(
        f::URL,
        f::START,
        f::headers()?,
        ConditionalRequestV1::Unconditional,
    )?;
    let response = f::observed_response()?;
    super::super::response(&work, &dispatch, &request, &response)?;
    let encoded = serde_json::to_vec(&response)?;
    let cold: ResponseObservationV1 = serde_json::from_slice(&encoded)?;
    check!(eq; cold, response);
    let raw = cold.observed_http().ok_or("response head")?.headers();
    check!(eq; raw.iter().find(|header| header.name() == "x-source").ok_or("raw response header")?.value(), &[128]);
    super::super::response(&work, &dispatch, &request, &cold)?;
    let old_response = ResponseObservationV1::new(200, f::URL, f::RECEIVED, Vec::new())?;
    let old_request = DispatchRequestV1::new(f::URL, ConditionalRequestV1::Unconditional)?;
    transition_refused(super::super::response(
        &work,
        &dispatch,
        &request,
        &old_response,
    ))?;
    transition_refused(super::super::response(
        &work,
        &dispatch,
        &old_request,
        &response,
    ))?;
    transition_refused(super::super::response(
        &f::whole()?,
        &dispatch,
        &request,
        &response,
    ))?;
    Ok(())
}

#[test]
fn response_url_and_response_head_chronology_are_bound_to_armed_request() -> TestResult {
    let work = f::public()?;
    let dispatch = f::dispatch(&work, 0)?;
    let request = f::request(
        f::URL,
        f::START,
        f::headers()?,
        ConditionalRequestV1::Unconditional,
    )?;
    let foreign = f::response(
        "https://mi.milesplit.com/athletes/13749841-other",
        f::RECEIVED,
        200,
        Vec::new(),
    )?;
    check!(matches!(
        super::super::response(&work, &dispatch, &request, &foreign),
        Err(StoreError::Capture(CaptureError::Domain(
            CaptureDomainError::RequestTargetMismatch
        )))
    ));
    let early = f::response(f::URL, "2026-10-21T07:59:59Z", 200, Vec::new())?;
    check!(matches!(
        super::super::response(&work, &dispatch, &request, &early),
        Err(StoreError::Capture(CaptureError::Domain(
            CaptureDomainError::Invalid {
                field: "observed hop chronology"
            }
        )))
    ));
    Ok(())
}

#[test]
fn selected_response_headers_authenticate_all_five_known_fields_once() -> TestResult {
    for (name, value) in [
        ("content-type", "text/html"),
        ("etag", "source-etag"),
        ("last-modified", "Wed, 21 Oct 2026 08:00:00 GMT"),
        ("location", "/athletes/13749840-new-slug"),
        ("retry-after", "30"),
    ] {
        let observed = ObservedHttpResponseV1::new(
            ObservedHttpProtocolV1::Http11,
            302,
            f::URL,
            f::RECEIVED,
            vec![ObservedHttpHeaderV1::new(name, value.as_bytes().to_vec())?],
        )?;
        ResponseObservationV1::new_observed(
            302,
            f::URL,
            f::RECEIVED,
            vec![(name.into(), value.into())],
            observed.clone(),
        )?;
        for selected in [Vec::new(), vec![(name.into(), "changed".into())]] {
            check!(matches!(
                ResponseObservationV1::new_observed(
                    302,
                    f::URL,
                    f::RECEIVED,
                    selected,
                    observed.clone(),
                ),
                Err(CaptureDomainError::Invalid {
                    field: "observed response selected headers"
                })
            ));
        }
        let duplicate = ObservedHttpResponseV1::new(
            ObservedHttpProtocolV1::Http11,
            302,
            f::URL,
            f::RECEIVED,
            vec![
                ObservedHttpHeaderV1::new(name, value.as_bytes().to_vec())?,
                ObservedHttpHeaderV1::new(name, value.as_bytes().to_vec())?,
            ],
        )?;
        check!(matches!(
            ResponseObservationV1::new_observed(
                302,
                f::URL,
                f::RECEIVED,
                vec![(name.into(), value.into())],
                duplicate,
            ),
            Err(CaptureDomainError::Invalid {
                field: "observed response selected headers"
            })
        ));
    }
    Ok(())
}
