use super::SummaryError;
use crate::net::CaptureTransportError;
use crate::CrawlError;

#[test]
fn boxed_summary_failure_preserves_typed_capture_error_and_source(
) -> Result<(), Box<dyn std::error::Error>> {
    let error = CrawlError::Capture(CaptureTransportError::GrantPoisoned);
    let detail = error.to_string();
    let summary = SummaryError::from(error);
    check!(eq; std::error::Error::source(&summary).ok_or("coach source")?.to_string(), detail);
    check!(eq; summary.to_string(), format!("coach mapping failed: {detail}"));
    check!(std::mem::size_of::<SummaryError>() < 128);
    let SummaryError::Coaches(error) = summary else {
        return Err("typed coach error lost".into());
    };
    check!(matches!(
        *error,
        CrawlError::Capture(CaptureTransportError::GrantPoisoned)
    ));
    Ok(())
}
