use super::*;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn an_oversized_cooldown_is_clamped_to_the_representable_maximum() -> TestResult {
    let rendered = cooldown_until_iso8601(u64::MAX);
    let maximum =
        chrono::DateTime::<chrono::Utc>::MAX_UTC.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    check!(
        eq;
        rendered,
        maximum,
        "an oversized cooldown clamps to the representable maximum"
    );
    let year = rendered
        .trim_start_matches('+')
        .split('-')
        .next()
        .ok_or("expanded year")?;
    check!(
        eq;
        year.len() >= 5,
        true,
        "the clamped instant leaves the four-digit year space: {rendered}"
    );
    Ok(())
}

#[test]
fn a_bounded_cooldown_renders_a_parseable_future_instant() -> TestResult {
    let rendered = cooldown_until_iso8601(3_600);
    let parsed = chrono::DateTime::parse_from_rfc3339(&rendered)?;
    check!(eq; parsed.timestamp() >= chrono::Utc::now().timestamp(), true);
    Ok(())
}
