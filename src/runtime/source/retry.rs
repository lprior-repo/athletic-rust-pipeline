//! What the transport still owns about retrying: the classification, not the retry.
//!
//! ADR-002 gives retrying one owner - the invocation retry declared on the handler - so the
//! transport performs a single attempt and reports what it saw. Nothing here asks the source
//! again: no attempt budget, no delay ladder, no sleep inside the invocation. What survives is the
//! part a caller needs to judge the attempt itself: which statuses are worth another attempt at
//! all. The `Retry-After` arithmetic a cooldown is built from lives in `athleticnet_browser::retry`
//! and is imported from there, so a browser cooldown and an HTTP lane read the same header the same
//! way.

/// Whether a status is worth another attempt, by a later invocation.
pub(crate) fn retryable_status(status: u16) -> bool {
    status == 429 || (500..=599).contains(&status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retries_only_rate_limit_and_server_failures() {
        assert!(retryable_status(429));
        assert!(retryable_status(503));
        assert!(!retryable_status(403));
        assert!(!retryable_status(400));
    }
}
