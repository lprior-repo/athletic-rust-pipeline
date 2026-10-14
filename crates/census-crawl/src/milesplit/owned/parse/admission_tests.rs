use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn state() -> Admission {
    Admission {
        frames: [Frame::default(); MAX_DEPTH],
        depth: 0,
        quoted: false,
        escaped: false,
        tokens: 0,
    }
}

#[test]
fn json_depth_admission_accepts_its_boundary_and_returns_the_exact_first_denied_byte() -> TestResult
{
    for depth in [MAX_DEPTH - 1, MAX_DEPTH] {
        let mut body = vec![b'['; depth];
        body.push(b'0');
        body.extend(vec![b']'; depth]);
        check!(eq; check(&body)?, ());
    }
    let body = vec![b'['; MAX_DEPTH + 1];
    match check(&body) {
        Err(OwnedParseResourceFailure::Limit {
            resource: OwnedParseResource::JsonDepth,
            locator: OwnedParseLocator::Byte { offset },
            requested,
            limit,
            retention: super::super::OwnedCaptureRetention::Unrecorded,
        }) => check!(eq; (offset, requested, limit), (127, 128, 127)),
        outcome => {
            return Err(format!("depth must return typed admission failure: {outcome:?}").into())
        }
    }
    Ok(())
}

#[test]
fn work_admission_accepts_at_the_token_bound_and_reports_the_next_origin() -> TestResult {
    let mut admission = state();
    admission.tokens = MAX_WORK - 2;
    let locator = OwnedParseLocator::Byte { offset: 37 };
    admission.accept(b',', locator)?;
    check!(eq; admission.tokens, MAX_WORK - 1);
    admission.accept(b',', locator)?;
    check!(eq; admission.tokens, MAX_WORK);
    match admission.accept(b',', locator) {
        Err(OwnedParseResourceFailure::Limit {
            resource: OwnedParseResource::DecodeWork,
            locator: found,
            requested,
            limit,
            retention: super::super::OwnedCaptureRetention::Unrecorded,
        }) => check!(eq; (found, requested, limit), (locator, MAX_WORK + 1, MAX_WORK)),
        outcome => {
            return Err(format!("work bound preserves exact typed cause: {outcome:?}").into())
        }
    }
    Ok(())
}

#[test]
fn decoded_byte_arithmetic_and_exact_limit_boundaries_are_checked() -> TestResult {
    let exact = decoded_bytes(MAX_DECODE_BYTES / TEXT_STORAGE_COPIES, 0, 0)?;
    check!(eq; exact, MAX_DECODE_BYTES);
    for requested in [exact - 1, exact] {
        check!(eq; OwnedParseResourceFailure::limit(OwnedParseResource::DecodedAllocation, OwnedParseLocator::Document, requested, MAX_DECODE_BYTES)?, ());
    }
    match OwnedParseResourceFailure::limit(
        OwnedParseResource::DecodedAllocation,
        OwnedParseLocator::Document,
        exact + 1,
        MAX_DECODE_BYTES,
    ) {
        Err(OwnedParseResourceFailure::Limit {
            resource: OwnedParseResource::DecodedAllocation,
            requested,
            limit,
            ..
        }) => {
            check!(eq; (requested, limit), (MAX_DECODE_BYTES + 1, MAX_DECODE_BYTES));
        }
        outcome => return Err(format!("decoded admission exceeded by one: {outcome:?}").into()),
    }
    for (body, nodes, rows, expected) in [
        (usize::MAX, 0, 0, OwnedParseArithmetic::TextBytes),
        (0, usize::MAX, 0, OwnedParseArithmetic::NodeBytes),
        (0, 0, usize::MAX, OwnedParseArithmetic::VectorBytes),
        (
            usize::MAX / TEXT_STORAGE_COPIES,
            1,
            0,
            OwnedParseArithmetic::TotalBytes,
        ),
    ] {
        let error =
            decoded_bytes(body, nodes, rows).map_err(super::super::OwnedMeetParseError::from);
        match error {
            Err(super::super::OwnedMeetParseError::Resource(
                OwnedParseResourceFailure::Arithmetic {
                    resource: OwnedParseResource::DecodedAllocation,
                    operation,
                    retention: super::super::OwnedCaptureRetention::Unrecorded,
                    ..
                },
            )) => check!(eq; operation, expected),
            outcome => {
                return Err(format!(
                    "arithmetic refusal is neither malformed nor page success: {outcome:?}"
                )
                .into())
            }
        }
    }
    Ok(())
}

#[test]
fn arithmetic_counter_refusals_keep_the_originating_resource_and_locator() -> TestResult {
    let locator = OwnedParseLocator::Byte { offset: 11 };
    let mut admission = state();
    admission.tokens = usize::MAX;
    match admission.accept(b',', locator) {
        Err(OwnedParseResourceFailure::Arithmetic {
            resource: OwnedParseResource::DecodeWork,
            locator: found,
            operation: OwnedParseArithmetic::TokenIncrement,
            ..
        }) => check!(eq; found, locator),
        outcome => return Err(format!("token increment preserves cause: {outcome:?}").into()),
    }
    let mut admission = state();
    admission.open(true, locator)?;
    admission.top().ok_or("array frame")?.values = usize::MAX;
    match admission.value(b'0', locator) {
        Err(OwnedParseResourceFailure::Arithmetic {
            resource: OwnedParseResource::ArrayValues,
            locator: found,
            operation: OwnedParseArithmetic::ArrayIncrement,
            ..
        }) => check!(eq; found, locator),
        outcome => return Err(format!("array increment preserves cause: {outcome:?}").into()),
    }
    Ok(())
}

#[test]
fn reservation_refusal_keeps_its_allocation_cause_without_allocating_rows() -> TestResult {
    let error = match super::super::reserved_page(usize::MAX) {
        Err(error) => super::super::OwnedMeetParseError::from(error),
        outcome => {
            return Err(format!("capacity refusal cannot create a page: {outcome:?}").into())
        }
    };
    match error {
        super::super::OwnedMeetParseError::Resource(OwnedParseResourceFailure::Allocation {
            resource: OwnedParseResource::AcceptedRows,
            locator: OwnedParseLocator::Data,
            requested,
            retention: super::super::OwnedCaptureRetention::Unrecorded,
            source,
        }) => {
            check!(eq; requested, usize::MAX);
            check!(eq; source.to_string(), "memory allocation failed because the computed capacity exceeded the collection's maximum");
        }
        other => {
            return Err(format!("allocation refusal must not become malformed: {other:?}").into())
        }
    }
    Ok(())
}
