use super::*;

#[test]
fn malformed_document_errors_keep_typed_original_locators_and_exact_legacy_bytes() -> TestResult {
    let cases: &[(&[u8], u64, OwnedParseLocator, &str)] = &[
        (
            b"{}",
            0,
            OwnedParseLocator::RequestedMeet,
            "invalid meet ID or oversized structured response",
        ),
        (
            b"<html>Forbidden</html>",
            725218,
            OwnedParseLocator::Document,
            "expected value at line 1 column 1",
        ),
        (
            b"{\"data\":null}",
            725218,
            OwnedParseLocator::Data,
            "expected a JSON object with a data array",
        ),
        (
            b"{\"_meta\":{\"status_code\":403},\"data\":[]}",
            725218,
            OwnedParseLocator::ProviderStatus,
            "provider envelope status is 403",
        ),
        (
            b"{\"_embedded\":{\"meet\":{\"id\":\"745082\"}},\"data\":[]}",
            725218,
            OwnedParseLocator::EmbeddedMeet,
            "embedded meet ID \"745082\" differs from requested meet 725218",
        ),
    ];
    for &(body, meet_id, expected_locator, detail) in cases {
        let failure = match parse_owned_meet(body, meet_id) {
            Err(OwnedMeetParseError::Malformed(failure)) => failure,
            outcome => return Err(format!("expected typed malformed source: {outcome:?}").into()),
        };
        let locator = match &failure {
            OwnedDocumentFailure::InvalidRequestedMeet { locator }
            | OwnedDocumentFailure::Json { locator, .. }
            | OwnedDocumentFailure::ProviderStatus { locator, .. }
            | OwnedDocumentFailure::EmbeddedMeet { locator, .. }
            | OwnedDocumentFailure::DataArray { locator } => *locator,
        };
        check!(eq; locator, expected_locator);
        check!(eq; failure.to_string(), detail);
        let verdict = OwnedMeetVerdict::Malformed {
            detail: failure.to_string(),
        };
        let historical = format!(
            "{{\"disposition\":\"malformed\",\"detail\":{}}}",
            serde_json::to_string(detail)?
        );
        check!(eq; serde_json::to_vec(&verdict)?, historical.as_bytes());
    }
    Ok(())
}

#[test]
fn structured_row_boundary_keeps_row_rejections_and_refuses_the_first_unadmitted_value(
) -> TestResult {
    for published in [MAX_OWNED_ROWS - 1, MAX_OWNED_ROWS] {
        let body = serde_json::to_vec(&json!({"data": vec![0; published]}))?;
        let page = parse_owned_meet(&body, 725218)?;
        check!(eq; page.published_rows, published);
        check!(eq; page.rows.len(), 0);
        check!(eq; page.rejected.len(), published);
        check!(eq; page.rejected.last().ok_or("last source row")?.locator, format!("data[{}]", published - 1));
    }
    let body = serde_json::to_vec(&json!({"data": vec![0; MAX_OWNED_ROWS + 1]}))?;
    match parse_owned_meet(&body, 725218) {
        Err(OwnedMeetParseError::Resource(OwnedParseResourceFailure::Limit {
            resource: OwnedParseResource::ArrayValues,
            locator: OwnedParseLocator::Byte { offset },
            requested,
            limit,
            retention: OwnedCaptureRetention::Unrecorded,
        })) => {
            check!(eq; (requested, limit, offset), (20_001, 20_000, 40_009));
        }
        outcome => {
            return Err(format!("row admission must remain a resource failure: {outcome:?}").into())
        }
    }
    Ok(())
}

#[test]
fn oversized_body_and_decoded_growth_are_resource_errors_even_before_json_decoding() -> TestResult {
    let body = vec![b' '; MAX_OWNED_BODY_BYTES + 1];
    match parse_owned_meet(&body, 725218) {
        Err(OwnedMeetParseError::Resource(OwnedParseResourceFailure::Limit {
            resource: OwnedParseResource::CaptureBytes,
            locator: OwnedParseLocator::Document,
            requested,
            limit,
            retention: OwnedCaptureRetention::Unrecorded,
        })) => check!(eq; (requested, limit), (MAX_OWNED_BODY_BYTES + 1, MAX_OWNED_BODY_BYTES)),
        outcome => {
            return Err(format!("oversized body is a typed resource stop: {outcome:?}").into())
        }
    }
    let body =
        serde_json::to_vec(&json!({"data": [], "providerExtension": "x".repeat(4 * 1024 * 1024)}))?;
    match parse_owned_meet(&body, 725218) {
        Err(OwnedMeetParseError::Resource(OwnedParseResourceFailure::Limit {
            resource: OwnedParseResource::DecodedAllocation,
            locator: OwnedParseLocator::Document,
            requested,
            limit,
            retention: OwnedCaptureRetention::Unrecorded,
        })) => {
            check!(eq; limit, 128 * 1024 * 1024);
            check!(requested > limit);
        }
        outcome => {
            return Err(format!("decoded growth is a typed resource stop: {outcome:?}").into())
        }
    }
    Ok(())
}

#[test]
fn large_envelope_diagnostics_refuse_resources_with_exact_encoded_need() -> TestResult {
    let body =
        serde_json::to_vec(&json!({"_meta": {"status_code": "é".repeat(2_000)}, "data": []}))?;
    match parse_owned_meet(&body, 725218) {
        Err(OwnedMeetParseError::Resource(OwnedParseResourceFailure::Limit {
            resource: OwnedParseResource::EnvelopeDiagnostic,
            locator: OwnedParseLocator::ProviderStatus,
            requested,
            limit,
            retention: OwnedCaptureRetention::Unrecorded,
        })) => check!(eq; (requested, limit), (4002, 1024)),
        outcome => {
            return Err(format!(
                "large envelope must not produce truncated malformed bytes: {outcome:?}"
            )
            .into())
        }
    }
    Ok(())
}

#[test]
fn public_parser_depth_boundary_agrees_with_the_pinned_json_decoder() -> TestResult {
    for nesting in [125, 126] {
        let body = format!(
            "{{\"data\":[],\"extension\":{}0{}}}",
            "[".repeat(nesting),
            "]".repeat(nesting)
        );
        let page = parse_owned_meet(body.as_bytes(), 725218)?;
        check!(eq; (page.rows.len(), page.rejected.len(), page.published_rows), (0, 0, 0));
    }
    let body = format!(
        "{{\"data\":[],\"extension\":{}0{}}}",
        "[".repeat(127),
        "]".repeat(127)
    );
    match parse_owned_meet(body.as_bytes(), 725218) {
        Err(OwnedMeetParseError::Resource(OwnedParseResourceFailure::Limit {
            resource: OwnedParseResource::JsonDepth,
            requested,
            limit,
            retention: OwnedCaptureRetention::Unrecorded,
            ..
        })) => check!(eq; (requested, limit), (128, 127)),
        outcome => {
            return Err(format!(
                "valid JSON beyond the decoder depth must be Resource: {outcome:?}"
            )
            .into())
        }
    }
    Ok(())
}

#[test]
fn envelope_diagnostic_budget_preserves_exact_bytes_below_and_at_the_bound() -> TestResult {
    for (text, encoded_bytes) in [("x".repeat(1021), 1023), ("é".repeat(511), 1024)] {
        let published = serde_json::to_string(&text)?;
        check!(eq; published.len(), encoded_bytes);
        let body = serde_json::to_vec(&json!({"_meta": {"status_code": text}, "data": []}))?;
        match parse_owned_meet(&body, 725218) {
            Err(OwnedMeetParseError::Malformed(OwnedDocumentFailure::ProviderStatus {
                published: actual,
                ..
            })) => {
                check!(eq; actual, published);
            }
            outcome => {
                return Err(format!(
                    "admitted diagnostic must preserve all original bytes: {outcome:?}"
                )
                .into())
            }
        }
    }
    let text = format!("{}x", "é".repeat(511));
    let body = serde_json::to_vec(&json!({"_meta": {"status_code": text}, "data": []}))?;
    match parse_owned_meet(&body, 725218) {
        Err(OwnedMeetParseError::Resource(OwnedParseResourceFailure::Limit {
            resource: OwnedParseResource::EnvelopeDiagnostic,
            locator: OwnedParseLocator::ProviderStatus,
            requested,
            limit,
            ..
        })) => check!(eq; (requested, limit), (1025, 1024)),
        outcome => {
            return Err(
                format!("first unadmitted UTF-8 byte must remain Resource: {outcome:?}").into(),
            )
        }
    }
    Ok(())
}
