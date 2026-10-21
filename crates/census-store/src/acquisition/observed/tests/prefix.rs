use super::{fixture as f, *};

#[test]
fn final_304_and_incomplete_prefixes_charge_exactly_16k_raw_header_bytes() -> TestResult {
    let work = f::public()?;
    let request = f::request(
        f::URL,
        f::START,
        f::headers()?,
        ConditionalRequestV1::Unconditional,
    )?;
    let request_bytes = bytes(request.observed_http().ok_or("request head")?.headers())?;
    for (total, accepted) in [(16_384usize, true), (16_385, false)] {
        let headers = padding(total.checked_sub(request_bytes).ok_or("response budget")?)?;
        check!(eq; request_bytes.checked_add(bytes(&headers)?), Some(total));
        let response = f::response(f::URL, f::RECEIVED, 304, headers)?;
        let mut facts = f::facts(f::dispatch(&work, 0)?, request.clone(), response.clone());
        outcome(super::super::prefix(&work, &facts, None, None), accepted)?;
        facts.dispatches.first_mut().ok_or("armed hop")?.1 = None;
        outcome(
            super::super::prefix(&work, &facts, None, Some(&response)),
            accepted,
        )?;
        facts.dispatches.clear();
        outcome(
            super::super::prefix(&work, &facts, Some(&request), Some(&response)),
            accepted,
        )?;
        let cold: ResponseObservationV1 = serde_json::from_slice(&serde_json::to_vec(&response)?)?;
        check!(eq; cold, response);
        outcome(
            super::super::prefix(&work, &facts, Some(&request), Some(&cold)),
            accepted,
        )?;
    }
    Ok(())
}

#[test]
fn redirect_prefix_counts_prior_response_and_each_armed_request_before_final_304() -> TestResult {
    let work = f::public()?;
    let request = f::request(
        f::URL,
        f::START,
        f::headers()?,
        ConditionalRequestV1::Unconditional,
    )?;
    let redirect = redirect_response(f::URL)?;
    let next = f::request(
        f::URL,
        f::RECEIVED,
        f::headers()?,
        ConditionalRequestV1::Unconditional,
    )?;
    let first_bytes = bytes(request.observed_http().ok_or("first head")?.headers())?;
    let redirect_bytes = bytes(redirect.observed_http().ok_or("redirect head")?.headers())?;
    let next_bytes = bytes(next.observed_http().ok_or("next head")?.headers())?;
    let head_bytes = first_bytes
        .checked_add(redirect_bytes)
        .and_then(|count| count.checked_add(next_bytes))
        .ok_or("prefix head bytes")?;
    for (total, accepted) in [(16_384usize, true), (16_385, false)] {
        let headers = padding(total.checked_sub(head_bytes).ok_or("final head budget")?)?;
        check!(eq; head_bytes.checked_add(bytes(&headers)?), Some(total));
        let final_response = f::response(f::URL, f::DONE, 304, headers)?;
        let mut facts = f::facts(f::dispatch(&work, 0)?, request.clone(), redirect.clone());
        outcome(
            super::super::prefix(&work, &facts, Some(&next), Some(&final_response)),
            accepted,
        )?;
        let later = f::facts(f::dispatch(&work, 1)?, next.clone(), final_response.clone());
        facts.dispatches.extend(later.dispatches);
        outcome(super::super::prefix(&work, &facts, None, None), accepted)?;
        facts.dispatches.last_mut().ok_or("last armed hop")?.1 = None;
        outcome(
            super::super::prefix(&work, &facts, None, Some(&final_response)),
            accepted,
        )?;
    }
    Ok(())
}

#[test]
fn redirect_to_304_requires_request_start_at_or_after_prior_response_head() -> TestResult {
    let work = f::public()?;
    let redirect = redirect_response(f::URL)?;
    let early = f::request(
        f::URL,
        f::START,
        f::headers()?,
        ConditionalRequestV1::Unconditional,
    )?;
    check!(matches!(
        super::super::redirect(&work, &redirect, &early),
        Err(StoreError::Capture(CaptureError::Domain(
            CaptureDomainError::Invalid {
                field: "observed chain chronology",
            }
        )))
    ));
    let next = f::request(
        f::URL,
        f::RECEIVED,
        f::headers()?,
        ConditionalRequestV1::Unconditional,
    )?;
    super::super::request(&work, &next)?;
    super::super::redirect(&work, &redirect, &next)?;
    let response = f::response(f::URL, f::DONE, 304, Vec::new())?;
    super::super::response(&work, &f::dispatch(&work, 1)?, &next, &response)?;
    let wrong_target = redirect_response("https://mi.milesplit.com/athletes/13749840-other-slug")?;
    check!(matches!(
        super::super::redirect(&work, &wrong_target, &next),
        Err(StoreError::Capture(CaptureError::Domain(
            CaptureDomainError::RequestTargetMismatch
        )))
    ));
    let not_redirect = f::response(f::URL, f::RECEIVED, 200, Vec::new())?;
    check!(matches!(
        super::super::redirect(&work, &not_redirect, &next),
        Err(StoreError::Capture(CaptureError::Domain(
            CaptureDomainError::Invalid {
                field: "observed redirect status",
            }
        )))
    ));
    Ok(())
}

#[test]
fn redirect_uses_unique_original_location_and_rejects_missing_or_duplicate_raw_fields() -> TestResult
{
    let work = f::public()?;
    let next = f::request(
        f::URL,
        f::RECEIVED,
        f::headers()?,
        ConditionalRequestV1::Unconditional,
    )?;
    let missing = f::response(f::URL, f::RECEIVED, 302, Vec::new())?;
    check!(matches!(
        super::super::redirect(&work, &missing, &next),
        Err(StoreError::Capture(CaptureError::Domain(
            CaptureDomainError::Invalid {
                field: "observed redirect location",
            }
        )))
    ));
    let raw = ObservedHttpResponseV1::new(
        ObservedHttpProtocolV1::Http2,
        302,
        f::URL,
        f::RECEIVED,
        vec![
            ObservedHttpHeaderV1::new("location", f::URL.as_bytes().to_vec())?,
            ObservedHttpHeaderV1::new("location", f::URL.as_bytes().to_vec())?,
        ],
    )?;
    check!(matches!(
        raw.validate_redirect_to(next.observed_http().ok_or("next head")?),
        Err(CaptureDomainError::Invalid {
            field: "observed redirect location"
        })
    ));
    Ok(())
}

#[test]
fn required_heads_cannot_be_missing_from_incomplete_prefix_or_redirect() -> TestResult {
    let mut fixture = f::fixture()?;
    let old_request = DispatchRequestV1::new(f::URL, ConditionalRequestV1::Unconditional)?;
    let old_response = ResponseObservationV1::new(302, f::URL, f::RECEIVED, Vec::new())?;
    transition_refused(super::super::redirect(
        &fixture.work,
        &old_response,
        &old_request,
    ))?;
    let original = &fixture
        .facts
        .dispatches
        .first()
        .ok_or("original hop")?
        .0
        .request;
    transition_refused(super::super::redirect(
        &fixture.work,
        &old_response,
        original,
    ))?;
    corrupt(super::super::prefix(
        &fixture.work,
        &fixture.facts,
        Some(&old_request),
        None,
    ))?;
    corrupt(super::super::prefix(
        &fixture.work,
        &fixture.facts,
        None,
        Some(&old_response),
    ))?;
    fixture
        .facts
        .dispatches
        .first_mut()
        .ok_or("armed hop")?
        .0
        .request = old_request;
    corrupt(super::super::prefix(
        &fixture.work,
        &fixture.facts,
        None,
        None,
    ))?;
    let whole = f::whole()?;
    let legacy_request = DispatchRequestV1::new(f::WHOLE, ConditionalRequestV1::Unconditional)?;
    let legacy_response = ResponseObservationV1::new(302, f::WHOLE, f::RECEIVED, Vec::new())?;
    let legacy = f::facts(
        f::dispatch(&whole, 0)?,
        legacy_request.clone(),
        legacy_response.clone(),
    );
    super::super::prefix(&whole, &legacy, None, None)?;
    super::super::redirect(&whole, &legacy_response, &legacy_request)?;
    Ok(())
}

fn redirect_response(target: &str) -> TestResult<ResponseObservationV1> {
    f::response(
        f::URL,
        f::RECEIVED,
        302,
        vec![ObservedHttpHeaderV1::new(
            "location",
            target.as_bytes().to_vec(),
        )?],
    )
}

fn bytes(headers: &[ObservedHttpHeaderV1]) -> TestResult<usize> {
    Ok(headers
        .iter()
        .try_fold(0usize, |count, header| {
            count
                .checked_add(header.name().len())
                .and_then(|count| count.checked_add(header.value().len()))
        })
        .ok_or("header byte overflow")?)
}

fn padding(mut bytes: usize) -> TestResult<Vec<ObservedHttpHeaderV1>> {
    let mut headers = Vec::new();
    for _ in 0..32 {
        if bytes == 0 {
            break;
        }
        let charge = bytes.min(2049);
        let length = charge.checked_sub(1).ok_or("padding name bytes")?;
        headers.push(ObservedHttpHeaderV1::new("x", vec![b'a'; length])?);
        bytes = bytes.checked_sub(charge).ok_or("padding charge")?;
    }
    check!(eq; bytes, 0);
    Ok(headers)
}

fn outcome(result: crate::StoreResult<()>, accepted: bool) -> TestResult {
    if accepted {
        result?;
    } else {
        check!(matches!(
            result,
            Err(StoreError::Capture(CaptureError::Domain(
                CaptureDomainError::Bound {
                    field: "observed chain header bytes",
                    max: 16384
                }
            )))
        ));
    }
    Ok(())
}

fn corrupt(result: crate::StoreResult<()>) -> TestResult {
    check!(matches!(
        result,
        Err(StoreError::Capture(CaptureError::CorruptClosure))
    ));
    Ok(())
}
