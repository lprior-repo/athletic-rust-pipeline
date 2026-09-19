use super::*;

#[test]
fn authentication_denial_is_not_a_transient_failure() {
    assert_eq!(
        status_code(StatusCode::UNAUTHORIZED),
        FailureCode::AccessDenied
    );
    assert_eq!(
        status_code(StatusCode::FORBIDDEN),
        FailureCode::AccessDenied
    );
    assert!(!retry::retryable_status(403));
    let denied = body_failure(
        FailureCode::Transport,
        StatusCode::FORBIDDEN,
        "truncated denial".to_owned(),
        Ok(Duration::ZERO),
    );
    assert!(!denied.retryable);
    let transient = body_failure(
        FailureCode::Transport,
        StatusCode::SERVICE_UNAVAILABLE,
        "truncated failure".to_owned(),
        Ok(Duration::ZERO),
    );
    assert!(transient.retryable);
}

#[test]
fn challenges_retain_evidence_for_browser_recovery_before_retry() -> anyhow::Result<()> {
    for (status, code, retryable) in [
        (
            StatusCode::TOO_MANY_REQUESTS,
            FailureCode::BrowserChallenge,
            true,
        ),
        (StatusCode::OK, FailureCode::BrowserChallenge, true),
        (StatusCode::FORBIDDEN, FailureCode::BrowserChallenge, true),
    ] {
        let receipt = DocumentReceipt {
            digest: crate::domain::identity::EvidenceDigest::parse(&"a".repeat(64))?,
            source_url: "http://127.0.0.1/challenge".to_owned(),
            http_status: status.as_u16(),
            media_type: "text/html".to_owned(),
            bytes: 1,
            fetched_at_unix_ms: 1,
            elapsed_ms: 1,
            rankings: None,
        };
        let result = outcome(status, receipt.clone(), Ok(Duration::ZERO), true);
        assert_eq!(result.code, Some(code));
        assert_eq!(result.retryable, retryable);
        assert_eq!(result.receipt, Some(receipt));
    }
    Ok(())
}
