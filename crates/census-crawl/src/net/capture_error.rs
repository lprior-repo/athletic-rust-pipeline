use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum CaptureTransportError {
    #[cfg(test)]
    #[error("fixture interruption at a durable capture boundary")]
    InjectedInterruption,
    #[error("the whole-meet continuation grant was already consumed")]
    GrantConsumed,
    #[error("the whole-meet continuation grant lock is poisoned")]
    GrantPoisoned,
    #[error("the requested whole meet differs from the service-admitted target")]
    TargetMismatch,
    #[error("eligible cache envelope is unsupported or malformed")]
    CacheEnvelopeUnsupported,
    #[error("historical cache has unknown physical-attempt provenance")]
    HistoricalAttemptUnknown,
    #[error("eligible cache does not match its original retained occurrence")]
    CacheOccurrenceMismatch,
    #[error("capture dispatch capability has been consumed")]
    DispatchConsumed,
    #[error("capture request differs from the immutable registered work")]
    RequestMismatch,
    #[error("capture admission requires the direct HTTP lane")]
    UnsupportedLane,
    #[error("capture work is interrupted")]
    Interrupted,
    #[error("capture work has a settled unsuccessful attempt")]
    Settled,
    #[error("capture backing cannot be read within its bound")]
    BackingUnavailable,
}
