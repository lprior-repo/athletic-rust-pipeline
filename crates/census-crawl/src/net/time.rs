use census_store::clock::{Clock, SystemClock};

pub fn now_iso8601() -> String {
    SystemClock.today_iso8601()
}

pub fn today_iso() -> String {
    SystemClock.today()
}

pub fn cooldown_until_iso8601(seconds: u64) -> String {
    let seconds = i64::try_from(seconds).unwrap_or(i64::MAX);
    chrono::Utc::now()
        .checked_add_signed(chrono::Duration::seconds(seconds))
        .unwrap_or(chrono::DateTime::<chrono::Utc>::MAX_UTC)
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub fn instant_iso8601(millis: u64) -> Option<String> {
    let millis = i64::try_from(millis).ok()?;
    chrono::DateTime::from_timestamp_millis(millis)
        .map(|instant| instant.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
}
