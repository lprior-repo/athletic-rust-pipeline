use super::*;
use crate::net::CaptureTransportError;
use crate::CrawlError;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn capture_fetch_variants_remain_failed_in_research_and_invariant_in_probe() -> TestResult {
    let errors = [
        FetchError::from(census_store::CaptureError::CacheRootMismatch),
        FetchError::CaptureDomain(census_domain::model::CaptureDomainError::Invalid {
            field: "response status",
        }),
        FetchError::Capture(CaptureTransportError::HistoricalAttemptUnknown),
    ];
    for error in errors {
        check!(eq; fetch(&error), Outcome::Failed);
        let error = CrawlError::Fetch(error);
        check!(eq; crawl(&error), Outcome::Failed);
        check!(!error.retryable());
        let (classification, detail) =
            crate::coach_directories::probe_utils::classify_error(&error);
        check!(eq; classification, "invariant");
        check!(eq; detail, error.to_string());
    }
    Ok(())
}

#[test]
fn capture_context_refusal_remains_failed_in_research_and_invariant_in_probe() -> TestResult {
    let errors = [
        CaptureTransportError::GrantPoisoned,
        CaptureTransportError::GrantConsumed,
        CaptureTransportError::TargetMismatch,
    ];
    for error in errors {
        let error = CrawlError::Capture(error);
        check!(eq; crawl(&error), Outcome::Failed);
        check!(!error.retryable());
        let (classification, detail) =
            crate::coach_directories::probe_utils::classify_error(&error);
        check!(eq; classification, "invariant");
        check!(eq; detail, error.to_string());
    }
    Ok(())
}
