use anyhow::{ensure, Context, Result};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::time::Duration;

use super::{clock_field, reported_realtime, validate_machine};

const MEASUREMENT_TOLERANCE: Duration = Duration::from_secs(1);

#[derive(Clone, Copy)]
struct ClockObservation {
    realtime: DateTime<Utc>,
    uptime: Duration,
}

pub(in super::super) struct ClockProgress<'a> {
    baseline: &'a Value,
    first: ClockObservation,
    previous: ClockObservation,
}

impl<'a> ClockProgress<'a> {
    pub(in super::super) fn new(baseline: &'a Value) -> Result<Self> {
        validate_machine(baseline, baseline)?;
        let first = parse_clock(baseline)?;
        Ok(Self {
            baseline,
            first,
            previous: first,
        })
    }

    pub(in super::super) fn observe(&mut self, clock: &Value) -> Result<bool> {
        validate_machine(self.baseline, clock)?;
        let sample = parse_clock(clock)?;
        validate_elapsed(self.previous, sample)?;
        validate_elapsed(self.first, sample)?;
        let crossed = super::crossed_day(
            self.first.realtime.date_naive(),
            sample.realtime.date_naive(),
        )?;
        self.previous = sample;
        Ok(crossed)
    }
}

fn parse_clock(clock: &Value) -> Result<ClockObservation> {
    let realtime = reported_realtime(clock)?;
    let raw = clock_field(clock, "monotonic_uptime")?;
    ensure!(
        raw.len() <= 64,
        "guest uptime exceeds 64-byte measurement budget"
    );
    let mut fields = raw.split_whitespace();
    let uptime = fields.next().context("guest monotonic uptime absent")?;
    let idle = fields.next().context("guest idle uptime absent")?;
    ensure!(fields.next().is_none(), "guest uptime has extra fields");
    let uptime = parse_fixed_decimal(uptime)?;
    parse_fixed_decimal(idle)?;
    Ok(ClockObservation { realtime, uptime })
}

fn parse_fixed_decimal(raw: &str) -> Result<Duration> {
    let (integer, fraction) = raw
        .split_once('.')
        .context("guest uptime decimal point absent")?;
    ensure!(
        !integer.is_empty() && integer.bytes().all(|byte| byte.is_ascii_digit()),
        "guest uptime seconds must be unsigned ASCII decimal"
    );
    ensure!(
        fraction.len() == 2 && fraction.bytes().all(|byte| byte.is_ascii_digit()),
        "guest uptime requires two ASCII fractional digits"
    );
    let seconds: u64 = integer.parse().context("guest uptime seconds overflow")?;
    let fraction: u32 = fraction.parse().context("guest uptime fraction invalid")?;
    let nanos = fraction
        .checked_mul(10_000_000)
        .context("guest uptime fraction overflow")?;
    Ok(Duration::new(seconds, nanos))
}

fn validate_elapsed(first: ClockObservation, sample: ClockObservation) -> Result<()> {
    let realtime_elapsed = sample
        .realtime
        .signed_duration_since(first.realtime)
        .to_std()
        .context("guest realtime regressed between validated observations")?;
    let uptime_elapsed = sample
        .uptime
        .checked_sub(first.uptime)
        .context("guest monotonic uptime regressed between validated observations")?;
    let discrepancy = realtime_elapsed.abs_diff(uptime_elapsed);
    ensure!(
        discrepancy <= MEASUREMENT_TOLERANCE,
        "guest realtime/uptime elapsed discrepancy {discrepancy:?} exceeds one-second measurement allowance"
    );
    Ok(())
}
