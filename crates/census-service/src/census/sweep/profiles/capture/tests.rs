use super::*;
use census_crawl::net::{CaptureTransportError, FetchError};
use census_store::StoreError;
type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[test]
fn profile_denials_cannot_enter_the_transient_retry_loop() -> TestResult {
    for (status, expected_refusal, expected_retry) in [
        (403, access::Refusal::Page, false),
        (429, access::Refusal::Host, false),
        (500, access::Refusal::None, true),
        (401, access::Refusal::None, false),
        (410, access::Refusal::None, false),
    ] {
        let outcome = classify(Err(CrawlError::Fetch(FetchError::Http {
            status,
            url: "https://mi.milesplit.com/athletes/9401".into(),
        })))?;
        let CaptureOutcome::SourceFailure {
            refusal, retryable, ..
        } = outcome
        else {
            return Err("ordinary source error must retain its classification".into());
        };
        check!(eq; refusal, expected_refusal);
        check!(eq; retryable, expected_retry);
    }
    let outcome = classify(Err(CrawlError::Fetch(FetchError::RateLimited {
        url: "https://mi.milesplit.com/athletes/9401".into(),
        retry_after_secs: Some(60),
    })))?;
    check!(matches!(
        outcome,
        CaptureOutcome::SourceFailure {
            refusal: access::Refusal::Host,
            retryable: false,
            ..
        }
    ));
    Ok(())
}

#[test]
fn settled_capture_and_nested_store_refusal_remain_errors_before_cursor_advance() -> TestResult {
    let settled = classify(Err(CrawlError::Fetch(FetchError::Capture(
        CaptureTransportError::Settled,
    ))));
    check!(matches!(
        settled,
        Err(CrawlError::Fetch(FetchError::Capture(
            CaptureTransportError::Settled
        )))
    ));
    let refused = classify(Err(CrawlError::Fetch(FetchError::CaptureStore(Box::new(
        StoreError::Refused {
            detail: "controlled original retention refusal".into(),
        },
    )))));
    let Err(CrawlError::Fetch(FetchError::CaptureStore(source))) = refused else {
        return Err("nested Store refusal must not become a source failure".into());
    };
    check!(
        matches!(*source, StoreError::Refused { ref detail } if detail == "controlled original retention refusal")
    );
    Ok(())
}

#[test]
fn recorded_cooldown_cannot_advance_as_an_ordinary_failed_profile() -> TestResult {
    let result = classify(Err(CrawlError::Fetch(FetchError::Cooldown {
        host: "mi.milesplit.com".into(),
    })));
    check!(
        matches!(result, Err(CrawlError::Fetch(FetchError::Cooldown { ref host })) if host == "mi.milesplit.com")
    );
    Ok(())
}

#[test]
fn profile_source_failure_message_is_bounded_at_a_utf8_boundary() -> TestResult {
    let result = source_failure(CrawlError::Fetch(FetchError::Http {
        status: 500,
        url: format!("https://mi.milesplit.com/{}", "é".repeat(4096)),
    }))?;
    let CaptureOutcome::SourceFailure {
        message, retryable, ..
    } = result
    else {
        return Err("ordinary source failure".into());
    };
    check!(message.len() <= 4096);
    check!(message.len() >= 4094);
    check!(retryable);
    Ok(())
}
