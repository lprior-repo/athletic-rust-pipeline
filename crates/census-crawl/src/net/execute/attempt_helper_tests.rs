use super::attempt_helper::retry_after_delay;
use chrono::{DateTime, TimeZone, Utc};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn now() -> TestResult<DateTime<Utc>> {
    let stamp = Utc.with_ymd_and_hms(1994, 11, 6, 8, 39, 37);
    Ok(stamp.single().ok_or("the fixed clock is valid")?)
}

#[test]
fn a_delay_seconds_value_is_taken_verbatim() -> TestResult {
    check!(eq; retry_after_delay("120", now()?), Some(120));
    check!(eq; retry_after_delay("0", now()?), Some(0));
    Ok(())
}

#[test]
fn an_imf_fixdate_becomes_the_remaining_delay() -> TestResult {
    check!(eq; retry_after_delay("Sun, 06 Nov 1994 08:49:37 GMT", now()?), Some(600));
    Ok(())
}

#[test]
fn an_rfc850_date_becomes_the_remaining_delay() -> TestResult {
    check!(eq; retry_after_delay("Sunday, 06-Nov-94 08:49:37 GMT", now()?), Some(600));
    Ok(())
}

#[test]
fn an_asctime_date_becomes_the_remaining_delay() -> TestResult {
    check!(eq; retry_after_delay("Sun Nov  6 08:49:37 1994", now()?), Some(600));
    Ok(())
}

#[test]
fn a_retroactive_date_is_an_explicit_zero_delay() -> TestResult {
    check!(eq; retry_after_delay("Sun, 06 Nov 1994 08:39:37 GMT", now()?), Some(0));
    check!(eq; retry_after_delay("Sat, 05 Nov 1994 08:49:37 GMT", now()?), Some(0));
    Ok(())
}

#[test]
fn an_unusable_value_is_absent_rather_than_zero() -> TestResult {
    for value in [
        "",
        " ",
        "soon",
        "-5",
        "12.5",
        "120s",
        "Sun, 32 Nov 1994 08:49:37 GMT",
        "Thu, 06 Nov 1994 25:49:37 GMT",
    ] {
        check!(eq; retry_after_delay(value, now()?), None, "value {value:?}");
    }
    Ok(())
}
