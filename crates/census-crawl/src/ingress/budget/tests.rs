use super::*;
use http::HeaderValue;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn response_classes_keep_literal_admission_limits_and_failure_precedence() -> TestResult {
    for (control, status, class, limit) in [
        (false, 200, ResponseClass::Data, 67_108_864),
        (true, 200, ResponseClass::Control, 65_536),
        (true, 202, ResponseClass::Control, 65_536),
        (false, 470, ResponseClass::Failure, 262_144),
        (true, 500, ResponseClass::Failure, 262_144),
    ] {
        let admission = Admission::new(control, StatusCode::from_u16(status)?, &HeaderMap::new());
        check!(eq; admission.class, class);
        check!(eq; admission.limit, limit);
    }
    Ok(())
}

#[test]
fn control_routes_are_classified_after_existing_route_rewriting() -> TestResult {
    for (path, expected) in [
        ("/restate/send/NationalCensus/selected/run", true),
        ("/restate/call/Census/publication_capabilities", true),
        ("/restate/call/BrowserSession/profile-0/fetch", false),
        ("/restate/call/Census/open_work", false),
        ("/restate/call/NationalCensus/selected/request", false),
        ("/restate/output/inv_abc", false),
        ("/restate/attach/inv_abc", false),
        ("/unknown/response", false),
    ] {
        let original: Uri = format!("http://localhost{path}").parse()?;
        let rewritten = super::super::invocation_route(&original).map_or(original, |value| value);
        check!(eq; is_control(&Method::POST, &rewritten), expected);
        check!(!is_control(&Method::GET, &rewritten));
    }
    Ok(())
}

#[test]
fn header_byte_and_duplicate_value_limits_admit_exact_boundary() -> TestResult {
    let mut headers = HeaderMap::new();
    headers.insert("x", HeaderValue::from_str(&"v".repeat(65_535))?);
    let admission = Admission::new(false, StatusCode::OK, &headers);
    admission.headers(&headers)?;
    headers.append("x", HeaderValue::from_static(""));
    let error = admission
        .headers(&headers)
        .err()
        .ok_or("oversized headers accepted")?;
    check!(matches!(
        error,
        IngressError::Refused(ResponseRefusal {
            kind: AdmissionKind::HeaderBytes,
            limit: 65_536,
            observed: 65_537,
            ..
        })
    ));
    let mut headers = HeaderMap::new();
    for _ in 0..256 {
        headers.append("x", HeaderValue::from_static(""));
    }
    admission.headers(&headers)?;
    headers.append("x", HeaderValue::from_static(""));
    let error = admission
        .headers(&headers)
        .err()
        .ok_or("too many headers accepted")?;
    check!(matches!(
        error,
        IngressError::Refused(ResponseRefusal {
            kind: AdmissionKind::HeaderCount,
            limit: 256,
            observed: 257,
            ..
        })
    ));
    Ok(())
}

#[test]
fn declared_length_refusal_keeps_original_status_and_bounded_invocation_id() -> TestResult {
    let mut headers = HeaderMap::new();
    headers.insert("x-restate-id", HeaderValue::from_static("inv_original123"));
    let admission = Admission::new(true, StatusCode::ACCEPTED, &headers).with_limit(8);
    admission.declared(None)?;
    admission.declared(Some(8))?;
    for declared in [9, u64::MAX] {
        let error = admission
            .declared(Some(declared))
            .err()
            .ok_or("oversized declaration accepted")?;
        let IngressError::Refused(refusal) = error else {
            return Err("wrong refusal type".into());
        };
        check!(eq; refusal.kind, AdmissionKind::DeclaredBodyBytes);
        check!(eq; refusal.status, 202);
        check!(eq; refusal.declared, Some(declared));
        check!(eq; refusal.invocation_id.as_deref(), Some("inv_original123"));
    }
    headers.insert("x-restate-id", HeaderValue::from_str(&"v".repeat(129))?);
    check!(Admission::new(false, StatusCode::OK, &headers)
        .invocation_id
        .is_none());
    Ok(())
}

#[test]
fn sdk_transport_wrapper_keeps_admission_and_allocation_refusals_distinct() -> TestResult {
    use restate_sdk::ingress::ClientError;
    let admission = Admission::new(false, StatusCode::OK, &HeaderMap::new());
    let refusal = admission.refusal(AdmissionKind::StreamedBodyBytes, 8, 9, None);
    let wrapped = ClientError::Transport {
        source: Box::new(IngressError::Refused(refusal)),
    };
    check!(super::super::is_admission_error(&wrapped));
    let source = Vec::<u8>::new()
        .try_reserve_exact(usize::MAX)
        .err()
        .ok_or("unrepresentable allocation accepted")?;
    let wrapped = ClientError::Transport {
        source: Box::new(IngressError::Allocation {
            refusal: admission.refusal(AdmissionKind::Allocation, 8, 9, None),
            source,
        }),
    };
    check!(super::super::is_admission_error(&wrapped));
    let source = reqwest::Client::builder()
        .user_agent("\n")
        .build()
        .err()
        .ok_or("invalid header accepted")?;
    let wrapped = ClientError::Transport {
        source: Box::new(IngressError::Transport(source)),
    };
    check!(!super::super::is_admission_error(&wrapped));
    check!(!super::super::is_admission_error(
        &ClientError::InvalidRequest {
            message: "unrelated".into()
        }
    ));
    Ok(())
}

#[test]
fn requested_invocation_retention_is_bounded_and_specific_to_output_and_attach() -> TestResult {
    for (path, expected) in [
        (
            "/restate/invocation/inv_original/output?ignored=yes".to_owned(),
            Some("inv_original"),
        ),
        (
            "/restate/invocation/inv_original/attach".to_owned(),
            Some("inv_original"),
        ),
        ("/restate/invocation/inv_original/other".to_owned(), None),
        (
            "/restate/invocation/extra/inv_original/output".to_owned(),
            None,
        ),
        ("/restate/invocation//output".to_owned(), None),
        (
            format!("/restate/invocation/{}/output", "x".repeat(129)),
            None,
        ),
    ] {
        let uri: Uri = path.parse()?;
        check!(eq; requested_invocation(&uri).as_deref(), expected);
    }
    let exact: Uri = format!("/restate/invocation/{}/output", "x".repeat(128)).parse()?;
    check!(eq; requested_invocation(&exact).map(|id| id.len()), Some(128));
    check!(eq; Admission::new(false, StatusCode::OK, &HeaderMap::new()).empty_chunks, 1024);
    Ok(())
}
