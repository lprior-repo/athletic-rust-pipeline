use super::attempt_helper::{
    recorded_millis, recorded_seconds, retry_after_delay, retry_after_header, retry_after_millis,
    RetryAfter, RetryAfterRefusal, MAX_RETRY_AFTER_SECONDS,
};
use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use reqwest::header::HeaderValue;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn now() -> TestResult<DateTime<Utc>> {
    let stamp = Utc.with_ymd_and_hms(1994, 11, 6, 8, 39, 37);
    Ok(stamp.single().ok_or("the fixed clock is valid")?)
}

fn advanced(now: DateTime<Utc>, millis: i64) -> TestResult<DateTime<Utc>> {
    let delta = TimeDelta::try_milliseconds(millis).ok_or("a representable skew delta")?;
    Ok(now
        .checked_add_signed(delta)
        .ok_or("a representable skew instant")?)
}

fn classified(value: &str) -> TestResult<RetryAfter> {
    let header = HeaderValue::from_bytes(value.as_bytes())?;
    Ok(retry_after_header(Some(&header), now()?))
}

#[test]
fn retry_after_the_four_outcomes_are_distinct() -> TestResult {
    let now = now()?;
    let absent = retry_after_header(None, now);
    let invalid = classified("soon")?;
    let delay = classified("120")?;
    let date = classified("Sun, 06 Nov 1994 08:49:37 GMT")?;
    check!(eq; absent, RetryAfter::Absent);
    check!(eq; invalid, RetryAfter::Refused(RetryAfterRefusal::Unparsable));
    check!(eq; delay, RetryAfter::Delay(120));
    check!(eq; date, RetryAfter::Deadline(advanced(now, 600_000)?));
    check!(ne; absent, invalid);
    check!(ne; delay, date);
    check!(eq; absent.seconds_at(now), None);
    check!(eq; invalid.seconds_at(now), None);
    check!(eq; delay.seconds_at(now), Some(120));
    check!(eq; date.seconds_at(now), Some(600));
    check!(eq; recorded_seconds(absent, now), None);
    check!(eq; recorded_seconds(delay, now), Some(120));
    Ok(())
}

#[test]
fn retry_after_a_missing_header_is_absent_and_never_zero() -> TestResult {
    let now = now()?;
    let absent = retry_after_header(None, now);
    check!(eq; absent, RetryAfter::Absent);
    check!(eq; absent.seconds_at(now), None);
    check!(eq; recorded_seconds(absent, now), None);
    check!(
        absent != RetryAfter::Delay(0),
        "absence is not a zero delay"
    );
    Ok(())
}

#[test]
fn retry_after_an_invalid_value_is_refused_not_absent() -> TestResult {
    let now = now()?;
    for value in [
        "",
        " ",
        "soon",
        "-5",
        "12.5",
        "+7",
        "120s",
        "Sun, 32 Nov 1994 08:49:37 GMT",
        "Thu, 06 Nov 1994 25:49:37 GMT",
    ] {
        check!(eq;
            retry_after_delay(value, now),
            RetryAfter::Refused(RetryAfterRefusal::Unparsable),
            "value {value:?}"
        );
    }
    let opaque = HeaderValue::from_bytes(&[0xff])?;
    check!(eq;
        retry_after_header(Some(&opaque), now),
        RetryAfter::Refused(RetryAfterRefusal::Unparsable),
        "a non-ASCII header value is unparsable, not absent"
    );
    check!(eq; recorded_seconds(retry_after_header(Some(&opaque), now), now), None);
    Ok(())
}

#[test]
fn retry_after_a_delta_seconds_value_is_the_checked_delay() -> TestResult {
    let now = now()?;
    check!(eq; retry_after_delay("120", now), RetryAfter::Delay(120));
    check!(eq; retry_after_delay("0", now), RetryAfter::Delay(0));
    check!(eq;
        retry_after_delay(&MAX_RETRY_AFTER_SECONDS.to_string(), now),
        RetryAfter::Delay(MAX_RETRY_AFTER_SECONDS),
        "the bound itself is honoured"
    );
    check!(eq;
        retry_after_delay(&MAX_RETRY_AFTER_SECONDS.saturating_add(1).to_string(), now),
        RetryAfter::Refused(RetryAfterRefusal::BeyondBound),
        "one second past the bound is refused"
    );
    Ok(())
}

#[test]
fn retry_after_a_future_http_date_is_its_own_deadline() -> TestResult {
    let now = now()?;
    for value in [
        "Sun, 06 Nov 1994 08:49:37 GMT",
        "Sunday, 06-Nov-94 08:49:37 GMT",
        "Sun Nov  6 08:49:37 1994",
    ] {
        let parsed = retry_after_delay(value, now);
        check!(eq;
            parsed,
            RetryAfter::Deadline(advanced(now, 600_000)?),
            "value {value:?}"
        );
        check!(eq;
            parsed.seconds_at(now),
            Some(600),
            "value {value:?} must never fall back to the missing-header default"
        );
        check!(eq; recorded_seconds(parsed, now), Some(600), "value {value:?}");
    }
    Ok(())
}

#[test]
fn retry_after_a_past_http_date_is_immediate_not_the_default() -> TestResult {
    let now = now()?;
    let equal = retry_after_delay("Sun, 06 Nov 1994 08:39:37 GMT", now);
    check!(eq; equal, RetryAfter::Deadline(now));
    check!(eq; equal.seconds_at(now), Some(0));
    check!(eq; recorded_seconds(equal, now), Some(0));
    let earlier = retry_after_delay("Sat, 05 Nov 1994 08:49:37 GMT", now);
    check!(
        matches!(earlier, RetryAfter::Deadline(_)),
        "a past date stays a typed deadline: {earlier:?}"
    );
    check!(eq; earlier.seconds_at(now), Some(0));
    check!(ne; recorded_seconds(earlier, now), recorded_seconds(RetryAfter::Absent, now));
    Ok(())
}

#[test]
fn retry_after_a_date_past_the_bound_is_refused() -> TestResult {
    let now = now()?;
    check!(eq;
        retry_after_delay("Mon, 07 Nov 1994 08:39:37 GMT", now),
        RetryAfter::Deadline(advanced(now, 86_400_000)?)
    );
    check!(eq;
        retry_after_delay("Mon, 07 Nov 1994 08:39:38 GMT", now),
        RetryAfter::Refused(RetryAfterRefusal::BeyondBound)
    );
    check!(eq;
        retry_after_delay("Sat, 06 Nov 2094 08:49:37 GMT", now),
        RetryAfter::Refused(RetryAfterRefusal::BeyondBound)
    );
    Ok(())
}

#[test]
fn retry_after_clock_skew_inside_a_second_is_immediate() -> TestResult {
    let now = now()?;
    check!(eq; RetryAfter::Deadline(advanced(now, 999)?).seconds_at(now), Some(0));
    check!(eq; RetryAfter::Deadline(advanced(now, 1_000)?).seconds_at(now), Some(1));
    check!(eq; RetryAfter::Deadline(advanced(now, -300_000)?).seconds_at(now), Some(0));
    check!(eq;
        recorded_seconds(RetryAfter::Deadline(advanced(now, -300_000)?), now),
        Some(0)
    );
    check!(eq; retry_after_millis(999, now), RetryAfter::Deadline(now));
    check!(eq; retry_after_millis(1_000, now), RetryAfter::Deadline(advanced(now, 1_000)?));
    Ok(())
}

#[test]
fn retry_after_an_oversized_value_is_refused_not_panicking() -> TestResult {
    let now = now()?;
    for value in [
        "9223372036854775807",
        "9223372036854775808",
        "18446744073709551615",
        "123456789012345678901234567890",
    ] {
        check!(eq;
            retry_after_delay(value, now),
            RetryAfter::Refused(RetryAfterRefusal::BeyondBound),
            "value {value:?}"
        );
    }
    check!(eq;
        retry_after_millis(u64::MAX, now),
        RetryAfter::Refused(RetryAfterRefusal::BeyondBound)
    );
    check!(eq; recorded_millis(u64::MAX), None);
    Ok(())
}

#[test]
fn retry_after_every_honoured_outcome_stays_inside_the_bound() -> TestResult {
    let now = now()?;
    let honoured = [
        retry_after_delay("0", now),
        retry_after_delay("1", now),
        retry_after_delay("120", now),
        retry_after_delay(&MAX_RETRY_AFTER_SECONDS.to_string(), now),
        retry_after_delay("Sun, 06 Nov 1994 08:49:37 GMT", now),
        retry_after_delay("Sunday, 06-Nov-94 08:49:37 GMT", now),
        retry_after_delay("Sun Nov  6 08:49:37 1994", now),
        retry_after_millis(30_000, now),
        retry_after_millis(MAX_RETRY_AFTER_SECONDS.saturating_mul(1_000), now),
    ];
    for delay in honoured {
        check!(
            delay
                .seconds_at(now)
                .is_none_or(|seconds| seconds <= MAX_RETRY_AFTER_SECONDS),
            "{delay:?}"
        );
    }
    check!(eq; RetryAfter::Delay(7).seconds_at(now), Some(7));
    check!(eq; RetryAfter::Absent.seconds_at(now), None);
    check!(eq; RetryAfter::Refused(RetryAfterRefusal::Unparsable).seconds_at(now), None);
    check!(eq; retry_after_millis(30_000, now).seconds_at(now), Some(30));
    Ok(())
}

#[test]
fn retry_after_millis_values_are_bounded_deadlines() -> TestResult {
    let now = now()?;
    check!(eq; retry_after_millis(0, now), RetryAfter::Deadline(now));
    check!(eq;
        retry_after_millis(30_000, now),
        RetryAfter::Deadline(advanced(now, 30_000)?)
    );
    check!(eq; retry_after_millis(30_000, now).seconds_at(now), Some(30));
    check!(eq; retry_after_millis(86_400_000, now).seconds_at(now), Some(86_400));
    check!(eq;
        retry_after_millis(86_401_000, now),
        RetryAfter::Refused(RetryAfterRefusal::BeyondBound)
    );
    check!(eq; recorded_millis(30_000), Some(30));
    Ok(())
}
