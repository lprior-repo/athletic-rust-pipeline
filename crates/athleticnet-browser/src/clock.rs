use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(test)]
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("system clock is outside the representable unix-millisecond range")]
pub struct ClockError;

pub trait Clock: Send + Sync {
    fn now_instant(&self) -> tokio::time::Instant;
    fn now_unix_ms(&self) -> Result<u64, ClockError>;
}

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

#[cfg(test)]
#[derive(Debug)]
pub(crate) struct TestClock {
    unix_ms: AtomicU64,
    monotonic_offset_ms: AtomicU64,
}

#[cfg(test)]
impl TestClock {
    pub(crate) fn at(unix_ms: u64) -> Self {
        Self {
            unix_ms: AtomicU64::new(unix_ms),
            monotonic_offset_ms: AtomicU64::new(0),
        }
    }

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
        match tokio::time::Instant::now().checked_add(offset) {
            Some(value) => value,
            None => tokio::time::Instant::now(),
        }
    }

    fn now_unix_ms(&self) -> Result<u64, ClockError> {
        Ok(self.unix_ms.load(Ordering::SeqCst))
    }
}

pub fn unix_ms_to_system_time(unix_ms: u64) -> Option<SystemTime> {
    UNIX_EPOCH.checked_add(Duration::from_millis(unix_ms))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_clock_reports_a_plausible_epoch() -> Result<(), Box<dyn std::error::Error>> {
        let clock = SystemClock;
        let ms = clock.now_unix_ms()?;
        check!(
            (1_577_836_800_000..4_102_444_800_000).contains(&ms),
            "unix ms {ms} is outside the plausible range"
        );
        Ok(())
    }

    #[test]
    fn system_clock_instant_is_monotonic() -> Result<(), Box<dyn std::error::Error>> {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                let clock = SystemClock;
                let first = clock.now_instant();
                let second = clock.now_instant();
                check!(second >= first, "monotonic instant went backwards");
                Ok(())
            })
    }

    #[test]
    fn test_clock_holds_still_until_advanced() -> Result<(), Box<dyn std::error::Error>> {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                let clock = TestClock::at(1_700_000_000_000);
                let first = clock.now_unix_ms()?;
                let first_instant = clock.now_instant();
                check!(eq; first, 1_700_000_000_000);
                check!(eq; clock.now_unix_ms()?, first);
                clock.advance_ms(1_500);
                check!(eq; clock.now_unix_ms()?, first + 1_500);
                check!(clock.now_instant() - first_instant >= Duration::from_millis(1_500));
                Ok(())
            })
    }
}
