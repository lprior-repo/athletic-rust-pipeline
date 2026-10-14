use super::*;

#[test]
fn typed_owned_parser_resource_failures_are_terminal_without_losing_their_origin() -> TestResult {
    use census_crawl::milesplit::{
        parse_owned_meet, OwnedCaptureRetention, OwnedMeetParseError, OwnedParseArithmetic,
        OwnedParseLocator, OwnedParseResource, OwnedParseResourceFailure,
    };
    let body =
        serde_json::to_vec(&serde_json::json!({"data": [], "providerExtension": vec![0; 20_001]}))?;
    let limit = match parse_owned_meet(&body, 725218) {
        Err(OwnedMeetParseError::Resource(
            error @ OwnedParseResourceFailure::Limit {
                resource: OwnedParseResource::ArrayValues,
                requested: 20_001,
                limit: 20_000,
                ..
            },
        )) => error,
        outcome => {
            return Err(
                format!("real owned parser must supply the resource cause: {outcome:?}").into(),
            )
        }
    };
    let mut rows = Vec::<u8>::new();
    let source = match rows.try_reserve_exact(usize::MAX) {
        Err(source) => source,
        Ok(()) => return Err("invalid capacity unexpectedly reserved".into()),
    };
    let arithmetic = OwnedParseResourceFailure::Arithmetic {
        resource: OwnedParseResource::DecodedAllocation,
        locator: OwnedParseLocator::Document,
        operation: OwnedParseArithmetic::TextBytes,
        retention: OwnedCaptureRetention::Unrecorded,
    };
    let allocation = OwnedParseResourceFailure::Allocation {
        resource: OwnedParseResource::AcceptedRows,
        locator: OwnedParseLocator::Data,
        requested: usize::MAX,
        source,
        retention: OwnedCaptureRetention::Unrecorded,
    };
    for failure in [limit, arithmetic, allocation] {
        let error = CrawlError::OwnedParseResource(failure);
        check!(!error.retryable());
        let originating_detail = error.to_string();
        match collect_error(error) {
            JobError::Terminal { message } => check!(eq; message, originating_detail),
            outcome => {
                return Err(format!(
                    "parser resource refusal must not retry acquisition: {outcome:?}"
                )
                .into())
            }
        }
    }
    Ok(())
}
