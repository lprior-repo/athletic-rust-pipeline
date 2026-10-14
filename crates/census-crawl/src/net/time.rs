use census_store::clock::{Clock, SystemClock};

pub fn now_iso8601() -> String {
    SystemClock.today_iso8601()
}

pub fn today_iso() -> String {
    SystemClock.today()
}

pub fn cooldown_until_iso8601(seconds: u64) -> String {
    let seconds = i64::try_from(seconds).map_or(i64::MAX, |value| value);
    chrono::TimeDelta::try_seconds(seconds)
        .and_then(|delta| chrono::Utc::now().checked_add_signed(delta))
        .map_or(chrono::DateTime::<chrono::Utc>::MAX_UTC, |value| value)
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub fn instant_iso8601(millis: u64) -> Option<String> {
    let millis = i64::try_from(millis).ok()?;
    chrono::DateTime::from_timestamp_millis(millis)
        .map(|instant| instant.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
}

pub(super) enum CaptureClock {
    System,
    #[cfg(test)]
    Injected(std::sync::Arc<dyn Clock>),
}

impl CaptureClock {
    pub(super) fn observed_at(&self) -> String {
        match self {
            Self::System => SystemClock.today_iso8601(),
            #[cfg(test)]
            Self::Injected(clock) => clock.today_iso8601(),
        }
    }
}
