//! Wall-clock helpers: the one place this crate reads the clock for evidence.
//!
//! Split out of `types.rs` so the fetch-statistics surface stays inside the file budget. The paths
//! are unchanged — `net` re-exports every name here — because their callers (cache writes, decode,
//! the collection default, the cooldown arithmetic) carry no clock of their own to inject.

use census_store::clock::{Clock, SystemClock};

/// Wall-clock timestamp for request evidence and cache metadata.
///
/// Delegates to the [`Clock`] capability so the crate has one source of wall-clock time: the
/// signatures stay as they are, because their callers — cache writes, decode, the collection
/// default — carry no clock of their own to inject.
pub fn now_iso8601() -> String {
    SystemClock.today_iso8601()
}

/// Today's date (`YYYY-MM-DD`), the default `observed_on` for a collection.
pub fn today_iso() -> String {
    SystemClock.today()
}

/// The instant a cooldown that starts now stops applying (RFC 3339 UTC, `Z`).
///
/// The one place a cooldown instant is computed, kept beside [`now_iso8601`] so the two timestamps a
/// condition carries are produced the same way. Clamped rather than panicking: a cooldown the clock
/// cannot represent is expressed as the far future, which is the honest reading of "blocked".
pub fn cooldown_until_iso8601(seconds: u64) -> String {
    let seconds = i64::try_from(seconds).unwrap_or(i64::MAX);
    chrono::Utc::now()
        .checked_add_signed(chrono::Duration::seconds(seconds))
        .unwrap_or(chrono::DateTime::<chrono::Utc>::MAX_UTC)
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// One instant read from unix milliseconds (RFC 3339 UTC, `Z`), when it is one this clock can state.
///
/// The browser lane's capture carries the instant it was taken as milliseconds, and the evidence
/// written from it wants the same shape the HTTP path writes: the format is [`now_iso8601`]'s, so a
/// receipt does not say which transport produced it. `None` for a value that is not an instant —
/// absence is then the caller's decision, the same way an absent `Retry-After` is.
pub fn instant_iso8601(millis: u64) -> Option<String> {
    let millis = i64::try_from(millis).ok()?;
    chrono::DateTime::from_timestamp_millis(millis)
        .map(|instant| instant.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
}
