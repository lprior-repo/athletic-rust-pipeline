/// Clock capability: the application's single source of time, kept behind a trait so deterministic
/// replay is possible: tests substitute a fixed date, production uses the system clock.
///
/// `today()` returns an ISO-8601 date (`YYYY-MM-DD`). `today_iso8601()` returns full ISO-8601
/// datetime. `now()` returns a monotonic instant for elapsed-time tracking — uses
/// `tokio::time::Instant` so `tokio::time::pause` drives it in tests.
///
/// The clock the application is allowed to read.
pub trait Clock: Send + Sync {
    /// ISO-8601 date, `YYYY-MM-DD`.
    fn today(&self) -> String;
    /// ISO-8601 datetime with timezone, `YYYY-MM-DDTHH:MM:SSZ`.
    fn today_iso8601(&self) -> String;
    /// Monotonic instant for elapsed-time tracking.
    fn now(&self) -> tokio::time::Instant;
}

/// System clock implementation.
pub struct SystemClock;

impl Clock for SystemClock {
    fn today(&self) -> String {
        chrono::Utc::now().date_naive().to_string()
    }

    fn today_iso8601(&self) -> String {
        chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    }

    fn now(&self) -> tokio::time::Instant {
        tokio::time::Instant::now()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_clock_today_format() {
        let clock = SystemClock;
        let today = clock.today();
        assert!(
            today.len() == 10 && today.chars().all(|c| c.is_ascii_digit() || c == '-'),
            "expected YYYY-MM-DD format, got {today}"
        );
    }

    #[test]
    fn system_clock_today_iso8601_format() {
        let clock = SystemClock;
        let dt = clock.today_iso8601();
        assert!(dt.ends_with("Z"), "expected Z timezone, got {dt}");
    }

    #[test]
    fn system_clock_now_is_monotonic() {
        let clock = SystemClock;
        let t1 = clock.now();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let t2 = clock.now();
        assert!(t2 >= t1);
    }
}
