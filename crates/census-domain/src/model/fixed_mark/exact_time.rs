use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(try_from = "TimeWire")]
pub struct ExactSeconds {
    nanoseconds: i64,
    precision: u8,
}

#[derive(Deserialize)]
struct TimeWire {
    nanoseconds: i64,
    precision: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TimeError {
    #[error("time must be a positive finite decimal with at most nine fractional digits")]
    Invalid,
    #[error("time exceeds the exact nanosecond range")]
    Overflow,
}

impl ExactSeconds {
    pub fn parse(raw: &str) -> Result<Self, TimeError> {
        let (nanoseconds, precision) = decimal_parts(raw)?;
        Self::from_parts(nanoseconds, precision)
    }

    pub fn parse_clock(raw: &str) -> Result<Self, TimeError> {
        let Some((prefix, seconds)) = raw.rsplit_once(':') else {
            return Self::parse(raw);
        };
        let (seconds, precision) = decimal_parts(seconds)?;
        if seconds >= 60_000_000_000 {
            return Err(TimeError::Invalid);
        }
        let nanoseconds = clock_minutes(prefix)?
            .checked_mul(60_000_000_000)
            .and_then(|value| value.checked_add(seconds))
            .ok_or(TimeError::Overflow)?;
        Self::from_parts(nanoseconds, precision)
    }

    pub fn from_parts(nanoseconds: i64, precision: u8) -> Result<Self, TimeError> {
        let exponent = 9_u32
            .checked_sub(u32::from(precision))
            .ok_or(TimeError::Invalid)?;
        let factor = 10_i64.checked_pow(exponent).ok_or(TimeError::Overflow)?;
        if nanoseconds <= 0 || nanoseconds.checked_rem(factor) != Some(0) {
            return Err(TimeError::Invalid);
        }
        Ok(Self {
            nanoseconds,
            precision,
        })
    }

    pub fn value(self) -> i64 {
        self.nanoseconds
    }

    pub fn precision(self) -> u8 {
        self.precision
    }

    pub fn try_as_seconds_f64(self) -> Option<f64> {
        rust_decimal::Decimal::new(self.nanoseconds, 9).to_f64()
    }
}

fn clock_minutes(raw: &str) -> Result<i64, TimeError> {
    let Some((hours, minutes)) = raw.split_once(':') else {
        return digits(raw);
    };
    let minutes = digits(minutes)?;
    if minutes >= 60 {
        return Err(TimeError::Invalid);
    }
    digits(hours)?
        .checked_mul(60)
        .and_then(|value| value.checked_add(minutes))
        .ok_or(TimeError::Overflow)
}

fn digits(raw: &str) -> Result<i64, TimeError> {
    if raw.is_empty() {
        return Err(TimeError::Invalid);
    }
    raw.bytes().try_fold(0_i64, |value, byte| {
        let digit = byte
            .checked_sub(b'0')
            .filter(|digit| *digit <= 9)
            .ok_or(TimeError::Invalid)?;
        value
            .checked_mul(10)
            .and_then(|value| value.checked_add(i64::from(digit)))
            .ok_or(TimeError::Overflow)
    })
}

fn decimal_parts(raw: &str) -> Result<(i64, u8), TimeError> {
    let (whole, fraction) = raw
        .split_once('.')
        .map_or((raw, None), |(whole, fraction)| (whole, Some(fraction)));
    let (fraction, precision) = fractional_parts(fraction)?;
    let factor = 10_i64
        .checked_pow(
            9_u32
                .checked_sub(u32::from(precision))
                .ok_or(TimeError::Invalid)?,
        )
        .ok_or(TimeError::Overflow)?;
    let nanoseconds = digits(whole)?
        .checked_mul(1_000_000_000)
        .and_then(|value| value.checked_add(fraction.checked_mul(factor)?))
        .ok_or(TimeError::Overflow)?;
    Ok((nanoseconds, precision))
}

fn fractional_parts(raw: Option<&str>) -> Result<(i64, u8), TimeError> {
    match raw {
        Some(fraction) if fraction.len() <= 9 => Ok((
            digits(fraction)?,
            u8::try_from(fraction.len()).map_err(|_| TimeError::Invalid)?,
        )),
        Some(_) => Err(TimeError::Invalid),
        None => Ok((0, 0)),
    }
}

impl PartialEq for ExactSeconds {
    fn eq(&self, other: &Self) -> bool {
        self.nanoseconds == other.nanoseconds
    }
}

impl Eq for ExactSeconds {}

impl Hash for ExactSeconds {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.nanoseconds.hash(state);
    }
}

impl TryFrom<TimeWire> for ExactSeconds {
    type Error = TimeError;

    fn try_from(value: TimeWire) -> Result<Self, Self::Error> {
        Self::from_parts(value.nanoseconds, value.precision)
    }
}

impl PartialOrd for ExactSeconds {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ExactSeconds {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.nanoseconds.cmp(&other.nanoseconds)
    }
}

impl std::fmt::Display for ExactSeconds {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut value = rust_decimal::Decimal::new(self.nanoseconds, 9);
        value.rescale(u32::from(self.precision));
        write!(formatter, "{value}")
    }
}

#[cfg(test)]
mod tests;
