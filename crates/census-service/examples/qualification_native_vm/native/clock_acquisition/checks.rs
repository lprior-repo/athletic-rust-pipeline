use super::{artifacts, FetchOutcome, FetchStats, Record, URL};
use anyhow::{ensure, Context, Result};
use chrono::{DateTime, FixedOffset};
use serde_json::Value;

pub(super) fn capture(capture: &FetchOutcome, stats: &FetchStats) -> Result<(usize, usize)> {
    ensure!(
        capture.url == URL && capture.method == "GET",
        "physical capture source mismatch"
    );
    ensure!(
        capture.status == 200 && !capture.from_cache,
        "fresh successful physical response required"
    );
    ensure!(
        capture.bytes == capture.body.len() && !capture.body.is_empty(),
        "capture byte count mismatch or empty body"
    );
    ensure!(
        artifacts::sha(&capture.body) == capture.content_digest,
        "capture SHA256 mismatch"
    );
    ensure!(
        stats.physical_requests() == 2
            && stats.cache_hits == 0
            && stats.conditional_304 == 0
            && stats.errors == 0
            && stats.bytes_downloaded == u64::try_from(capture.bytes)?,
        "acquisition must contain one robots policy request and one new full-body response: {stats:?}"
    );
    let html = std::str::from_utf8(&capture.body).context("RIIL directory is not UTF-8")?;
    ensure!(
        html.matches("<details").count() == html.matches("</details>").count(),
        "RIIL school sections are truncated"
    );
    let schools = census_crawl::riil::parse_directory(html);
    let appointments = schools.iter().try_fold(0_usize, |count, school| {
        count
            .checked_add(school.coach_rows.len())
            .context("RIIL appointment count overflow")
    })?;
    ensure!(
        !schools.is_empty() && appointments > 0,
        "physical response has no real RIIL XC/TF directory data"
    );
    Ok((schools.len(), appointments))
}

pub(super) fn acquisition_time(
    capture: &FetchOutcome,
    before: &Value,
    after: &Value,
) -> Result<()> {
    let start = instant(before)?;
    let end = instant(after)?;
    let fetched = DateTime::parse_from_rfc3339(&capture.fetched_at)
        .context("production fetched_at is not RFC3339")?;
    ensure!(
        fetched.offset().local_minus_utc() == 0,
        "capture timestamp must be UTC"
    );
    ensure!(
        start <= end && fetched.timestamp() >= start.timestamp() && fetched <= end,
        "production capture timestamp is outside measured guest acquisition interval"
    );
    let day = fetched.date_naive().to_string();
    ensure!(
        field(before, "date")? == day && field(after, "date")? == day,
        "acquisition crossed a day internally or capture date is dishonest"
    );
    ensure!(
        field(before, "boot_id")? == field(after, "boot_id")?,
        "acquisition guest rebooted"
    );
    ensure!(
        field(before, "machine_id")? == field(after, "machine_id")?,
        "acquisition machine changed"
    );
    ensure!(
        uptime(after)? >= uptime(before)?,
        "guest monotonic uptime reversed during acquisition"
    );
    Ok(())
}

pub(super) fn crossing(before: &Record, after: &Record) -> Result<()> {
    ensure!(
        before.identity == after.identity,
        "run/cohort/season/source-unit identity changed across midnight"
    );
    ensure!(
        before.phase == super::Phase::Before && after.phase == super::Phase::After,
        "acquisition phase sequence invalid"
    );
    let first = DateTime::parse_from_rfc3339(&before.capture.fetched_at)?;
    let second = DateTime::parse_from_rfc3339(&after.capture.fetched_at)?;
    ensure!(
        first < second && first.date_naive().succ_opt() == Some(second.date_naive()),
        "fresh physical fetched_at dates did not cross exactly one natural UTC midnight"
    );
    ensure!(
        field(&before.clock, "boot_id")? == field(&after.clock_before, "boot_id")?,
        "midnight acquisition proof rebooted guest"
    );
    ensure!(
        field(&before.clock, "machine_id")? == field(&after.clock_before, "machine_id")?,
        "midnight acquisition machine differs"
    );
    ensure!(
        uptime(&after.clock_before)? > uptime(&before.clock)?,
        "midnight acquisition uptime did not advance"
    );
    Ok(())
}

fn instant(clock: &Value) -> Result<DateTime<FixedOffset>> {
    Ok(DateTime::parse_from_rfc3339(field(clock, "realtime")?)?)
}

fn uptime(clock: &Value) -> Result<f64> {
    let value: f64 = field(clock, "monotonic_uptime")?
        .split_whitespace()
        .next()
        .context("guest uptime absent")?
        .parse()?;
    ensure!(value.is_finite() && value >= 0.0, "guest uptime invalid");
    Ok(value)
}

fn field<'a>(clock: &'a Value, name: &str) -> Result<&'a str> {
    clock
        .get(name)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .with_context(|| format!("measured guest clock {name} absent"))
}
