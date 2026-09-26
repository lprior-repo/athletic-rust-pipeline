//! Time as an injected capability: one seam, one spelling, one place to virtualize.
//!
//! Every reading of time in the transport goes through [`Clock`]. Two methods, because the
//! transport needs two different kinds of time and conflating them is a correctness bug:
//!
//! - [`Clock::now_instant`] is monotonic and returns `tokio::time::Instant` — the only instant
//!   type `tokio::time::pause`/`advance` can virtualize. Deadlines, elapsed-time measurements,
//!   cooldowns and timeouts MUST use it, or a paused test silently exercises the wrong branch.
//! - [`Clock::now_unix_ms`] is wall clock and returns milliseconds since the Unix epoch. Anything a
//!   human or another machine will read back — HTTP `Retry-After` arithmetic, receipt timestamps —
//!   MUST use it, because a monotonic instant has no absolute meaning.
//!
//! `SystemClock` is the only production implementation; `TestClock` is the deterministic double.
//! The pipeline crate injects the same trait and keeps only its Restate-journaled reading
//! (`journal_unix_ms`), which needs an object context this crate must not depend on.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(test)]
use std::sync::atomic::{AtomicU64, Ordering};

/// The system clock could not produce a unix-millisecond reading.
///
/// Unreachable on any host whose clock is after 1970 and before year 584942417, which is why it is
/// a distinct type rather than an `anyhow` string: callers that can degrade do so explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("system clock is outside the representable unix-millisecond range")]
pub struct ClockError;

/// Pure time reading, injected rather than read ambiently.
pub trait Clock: Send + Sync {
    /// Monotonic instant, virtualizable by `tokio::time::pause`.
    fn now_instant(&self) -> tokio::time::Instant;
    /// Wall-clock milliseconds since the Unix epoch.
    fn now_unix_ms(&self) -> Result<u64, ClockError>;
}

/// The production clock: reads the operating system directly.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_instant(&self) -> tokio::time::Instant {
        tokio::time::Instant::now()
    }

    fn now_unix_ms(&self) -> Result<u64, ClockError> {
        let elapsed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ClockError)?;
        u64::try_from(elapsed.as_millis()).map_err(|_| ClockError)
    }
}

/// Deterministic clock for tests: both readings move only when the test moves them.
///
/// The monotonic side is an offset applied to `tokio::time::Instant::now()` rather than a value
/// captured at construction, so a `TestClock` still tracks a paused or advanced runtime and
/// `timeout_at(clock.now_instant() + budget)` keeps meaning "budget from now".
///
/// Compiled only for tests: production code resolves one clock per manager, and an injectable
/// double that no production path can reach keeps the non-test build free of dead code.
#[cfg(test)]
#[derive(Debug)]
pub(crate) struct TestClock {
    unix_ms: AtomicU64,
    monotonic_offset_ms: AtomicU64,
}

#[cfg(test)]
impl TestClock {
    /// A clock frozen at `unix_ms` with a zero monotonic offset.
    pub(crate) fn at(unix_ms: u64) -> Self {
        Self {
            unix_ms: AtomicU64::new(unix_ms),
            monotonic_offset_ms: AtomicU64::new(0),
        }
    }

    /// Move both readings forward by `by` milliseconds.
    pub(crate) fn advance_ms(&self, by: u64) {
        let _ = self
            .unix_ms
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                Some(current.saturating_add(by))
            });
        let _ =
            self.monotonic_offset_ms
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                    Some(current.saturating_add(by))
                });
    }
}

#[cfg(test)]
impl Clock for TestClock {
    fn now_instant(&self) -> tokio::time::Instant {
        let offset = Duration::from_millis(self.monotonic_offset_ms.load(Ordering::SeqCst));
        tokio::time::Instant::now()
            .checked_add(offset)
            .unwrap_or_else(tokio::time::Instant::now)
    }

    fn now_unix_ms(&self) -> Result<u64, ClockError> {
        Ok(self.unix_ms.load(Ordering::SeqCst))
    }
}

/// Convert a wall-clock reading into the `SystemTime` the HTTP-date parsers work in.
///
/// `None` only when `unix_ms` is beyond what `SystemTime` can represent; callers on a retry path
/// fail closed on it rather than substituting a different instant.
pub fn unix_ms_to_system_time(unix_ms: u64) -> Option<SystemTime> {
    UNIX_EPOCH.checked_add(Duration::from_millis(unix_ms))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_clock_reports_a_plausible_epoch() {
        let clock = SystemClock;
        let ms = clock.now_unix_ms().expect("system clock is representable");
        assert!(
            (1_577_836_800_000..4_102_444_800_000).contains(&ms),
            "unix ms {ms} is outside the plausible range"
        );
    }

    #[tokio::test]
    async fn system_clock_instant_is_monotonic() {
        let clock = SystemClock;
        let first = clock.now_instant();
        let second = clock.now_instant();
        assert!(second >= first, "monotonic instant went backwards");
    }

    #[tokio::test]
    async fn test_clock_holds_still_until_advanced() {
        let clock = TestClock::at(1_700_000_000_000);
        let first = clock.now_unix_ms().expect("test clock is infallible");
        let first_instant = clock.now_instant();
        assert_eq!(first, 1_700_000_000_000);
        assert_eq!(
            clock.now_unix_ms().expect("test clock is infallible"),
            first
        );
        clock.advance_ms(1_500);
        assert_eq!(
            clock.now_unix_ms().expect("test clock is infallible"),
            first + 1_500
        );
        assert!(clock.now_instant() - first_instant >= Duration::from_millis(1_500));
    }
}
