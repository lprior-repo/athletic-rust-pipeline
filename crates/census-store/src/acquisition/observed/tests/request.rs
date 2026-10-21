use super::{fixture as f, *};

#[test]
fn original_whole_meet_without_observed_http_remains_accepted() -> TestResult {
    let work = f::whole()?;
    let request = DispatchRequestV1::new(f::WHOLE, ConditionalRequestV1::Unconditional)?;
    super::super::request(&work, &request)?;
    let response = ResponseObservationV1::new(200, f::WHOLE, f::RECEIVED, Vec::new())?;
    let dispatch = f::dispatch(&work, 0)?;
    super::super::response(&work, &dispatch, &request, &response)?;
    let facts = f::facts(dispatch, request, response);
    let prepared = PreparedCaptureInputV1::new(&"c".repeat(64), 4, b"{}".to_vec())?;
    super::super::metadata(&work, &facts, &prepared, None)?;
    Ok(())
}

#[test]
fn public_work_requires_exact_observed_head_and_old_work_refuses_it() -> TestResult {
    let public = f::public()?;
    transition_refused(super::super::request(
        &public,
        &DispatchRequestV1::new(f::URL, ConditionalRequestV1::Unconditional)?,
    ))?;
    let request = f::request(
        f::URL,
        f::START,
        f::headers()?,
        ConditionalRequestV1::Unconditional,
    )?;
    super::super::request(&public, &request)?;
    let reopened: DispatchRequestV1 = serde_json::from_slice(&serde_json::to_vec(&request)?)?;
    check!(eq; reopened, request);
    super::super::request(&public, &reopened)?;
    transition_refused(super::super::request(&f::whole()?, &request))?;
    Ok(())
}

#[test]
fn representation_headers_require_exact_values_and_one_occurrence_without_extras() -> TestResult {
    let work = f::public()?;
    let baseline = f::headers()?;
    let mut missing = baseline.clone();
    missing.pop().ok_or("user agent header")?;
    let mut extra = baseline.clone();
    extra.push(ObservedHttpHeaderV1::new("accept", b"text/html".to_vec())?);
    let mut duplicate = baseline.clone();
    duplicate.push(ObservedHttpHeaderV1::new(
        "user-agent",
        b"Census/1.0".to_vec(),
    )?);
    let changed = vec![
        ObservedHttpHeaderV1::new("accept-encoding", b"identity".to_vec())?,
        ObservedHttpHeaderV1::new("user-agent", b"Different/1.0".to_vec())?,
    ];
    let unexpected_validator = vec![
        ObservedHttpHeaderV1::new("accept-encoding", b"identity".to_vec())?,
        ObservedHttpHeaderV1::new("user-agent", b"Census/1.0".to_vec())?,
        ObservedHttpHeaderV1::new("if-none-match", b"source-etag".to_vec())?,
    ];
    for headers in [missing, extra, duplicate, changed, unexpected_validator] {
        let request = f::request(
            f::URL,
            f::START,
            headers,
            ConditionalRequestV1::Unconditional,
        )?;
        transition_refused(super::super::request(&work, &request))?;
    }
    Ok(())
}

#[test]
fn pinned_validators_are_exact_single_occurrences_and_absent_validators_stay_absent() -> TestResult
{
    let work = f::public()?;
    let conditional = ConditionalRequestV1::Pinned {
        original: AcquisitionOccurrenceRefV1::new(
            f::dispatch(&work, 0)?.attempt().clone(),
            AcquisitionDigest::parse(&"d".repeat(64))?,
        ),
        etag: Some("source-etag".into()),
        last_modified: Some("Wed, 21 Oct 2026 08:00:00 GMT".into()),
    };
    let mut valid = f::headers()?;
    valid.push(ObservedHttpHeaderV1::new(
        "if-none-match",
        b"source-etag".to_vec(),
    )?);
    valid.push(ObservedHttpHeaderV1::new(
        "if-modified-since",
        b"Wed, 21 Oct 2026 08:00:00 GMT".to_vec(),
    )?);
    super::super::request(
        &work,
        &f::request(f::URL, f::START, valid.clone(), conditional.clone())?,
    )?;
    let mut missing = valid.clone();
    missing.pop().ok_or("last modified validator")?;
    let mut changed = valid.clone();
    *changed
        .iter_mut()
        .find(|header| header.name() == "if-none-match")
        .ok_or("etag validator")? =
        ObservedHttpHeaderV1::new("if-none-match", b"changed".to_vec())?;
    let mut duplicate = valid;
    duplicate.push(ObservedHttpHeaderV1::new(
        "if-none-match",
        b"source-etag".to_vec(),
    )?);
    for headers in [f::headers()?, missing, duplicate, changed] {
        let request = f::request(f::URL, f::START, headers, conditional.clone())?;
        transition_refused(super::super::request(&work, &request))?;
    }
    Ok(())
}

#[test]
fn a_pinned_etag_does_not_authorize_an_unpinned_last_modified_header() -> TestResult {
    let work = f::public()?;
    let conditional = ConditionalRequestV1::Pinned {
        original: AcquisitionOccurrenceRefV1::new(
            f::dispatch(&work, 0)?.attempt().clone(),
            AcquisitionDigest::parse(&"d".repeat(64))?,
        ),
        etag: Some("source-etag".into()),
        last_modified: None,
    };
    let mut headers = f::headers()?;
    headers.push(ObservedHttpHeaderV1::new(
        "if-none-match",
        b"source-etag".to_vec(),
    )?);
    let valid = f::request(f::URL, f::START, headers.clone(), conditional.clone())?;
    super::super::request(&work, &valid)?;
    headers.push(ObservedHttpHeaderV1::new(
        "if-modified-since",
        b"Wed, 21 Oct 2026 08:00:00 GMT".to_vec(),
    )?);
    let extra = f::request(f::URL, f::START, headers, conditional)?;
    transition_refused(super::super::request(&work, &extra))?;
    Ok(())
}

#[test]
fn same_origin_redirect_cannot_dispatch_another_native_subject_or_authentication_path() -> TestResult
{
    let work = f::public()?;
    let original = f::request(
        f::URL,
        f::START,
        f::headers()?,
        ConditionalRequestV1::Unconditional,
    )?;
    let dispatch = f::dispatch(&work, 0)?;
    for target in [
        "https://mi.milesplit.com/athletes/13749841-other",
        "https://mi.milesplit.com/login",
        "https://mi.milesplit.com/auth/13749840",
        "https://mn.milesplit.com/athletes/13749840-brennan-smith",
    ] {
        let redirect = f::response(
            f::URL,
            f::RECEIVED,
            302,
            vec![ObservedHttpHeaderV1::new(
                "location",
                target.as_bytes().to_vec(),
            )?],
        )?;
        super::super::response(&work, &dispatch, &original, &redirect)?;
        let next = f::request(
            target,
            f::RECEIVED,
            f::headers()?,
            ConditionalRequestV1::Unconditional,
        )?;
        check!(matches!(
            super::super::request(&work, &next),
            Err(StoreError::Capture(CaptureError::Domain(
                CaptureDomainError::RequestTargetMismatch
            )))
        ));
    }
    let same_subject = f::request(
        "https://mi.milesplit.com/athletes/13749840-new-slug",
        f::RECEIVED,
        f::headers()?,
        ConditionalRequestV1::Unconditional,
    )?;
    super::super::request(&work, &same_subject)?;
    Ok(())
}
