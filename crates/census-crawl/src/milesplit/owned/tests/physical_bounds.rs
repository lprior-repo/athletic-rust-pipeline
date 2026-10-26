use super::*;

#[test]
fn body_byte_boundary_admits_capture_size_before_the_independent_decoded_budget_refuses(
) -> TestResult {
    let vector_bytes = MAX_OWNED_ROWS
        .checked_mul(
            std::mem::size_of::<OwnedPerformance>()
                .checked_add(std::mem::size_of::<OwnedRejection>())
                .ok_or("row size")?,
        )
        .ok_or("vector size")?;
    for length in [MAX_OWNED_BODY_BYTES - 1, MAX_OWNED_BODY_BYTES] {
        let body = vec![b' '; length];
        let expected = length
            .checked_mul(32)
            .and_then(|bytes| bytes.checked_add(vector_bytes))
            .ok_or("decoded need")?;
        match parse_owned_meet(&body, 725218) {
            Err(OwnedMeetParseError::Resource(OwnedParseResourceFailure::Limit {
                resource: OwnedParseResource::DecodedAllocation,
                locator: OwnedParseLocator::Document,
                requested,
                limit,
                ..
            })) => check!(eq; (requested, limit), (expected, 512 * 1024 * 1024)),
            outcome => {
                return Err(format!(
                    "capture admitted within its bound, then decoded budget refused: {outcome:?}"
                )
                .into())
            }
        }
    }
    Ok(())
}

#[test]
fn public_parser_work_limit_preserves_the_first_original_byte_and_exact_budget() -> TestResult {
    let prefix = "{\"data\":[],\"extension\":\"";
    let body = format!("{prefix}{}\"}}", ",".repeat(2_000_000));
    let offset = prefix
        .len()
        .checked_add(2_000_000 - 11)
        .ok_or("work offset")?;
    match parse_owned_meet(body.as_bytes(), 725218) {
        Err(OwnedMeetParseError::Resource(OwnedParseResourceFailure::Limit {
            resource: OwnedParseResource::DecodeWork,
            locator: OwnedParseLocator::Byte { offset: found },
            requested,
            limit,
            ..
        })) => {
            check!(eq; (found, requested, limit), (offset, 2_000_001, 2_000_000));
            check!(eq; body.as_bytes().get(found), Some(&b','));
        }
        outcome => {
            return Err(format!("public work refusal must remain resource: {outcome:?}").into())
        }
    }
    Ok(())
}
