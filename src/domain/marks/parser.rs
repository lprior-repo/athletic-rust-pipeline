use super::{Milliseconds, Nanometers};
use crate::domain::error::DomainError;

pub(super) fn parse_time(raw: &str) -> Result<Milliseconds, DomainError> {
    let trimmed = raw.trim();
    let value = match trimmed
        .strip_suffix('s')
        .or_else(|| trimmed.strip_suffix('S'))
    {
        Some(value) => value.trim(),
        None => trimmed,
    };
    if let Some((minutes, seconds)) = value.split_once(':') {
        let mins = parse_integer(minutes, "mark")?;
        let secs = decimal_scaled(seconds, 1_000, "mark")?;
        if secs >= 60_000 {
            return Err(DomainError::OutOfRange { field: "mark" });
        }
        return mins
            .checked_mul(60_000)
            .and_then(|value| value.checked_add(secs))
            .and_then(|value| positive(value, "mark").ok())
            .map(Milliseconds)
            .ok_or(DomainError::OutOfRange { field: "mark" });
    }
    decimal_scaled(value, 1_000, "mark")
        .and_then(|value| positive(value, "mark"))
        .map(Milliseconds)
}

pub(super) fn parse_distance(raw: &str) -> Result<Nanometers, DomainError> {
    let value = raw.trim();
    if let Some((feet, inches)) = value.split_once('\'').or_else(|| value.split_once('-')) {
        let feet_value = parse_integer(feet.trim(), "mark")?;
        let inches = inches.trim();
        let inch_text = inches.strip_suffix('"').map_or(inches, str::trim);
        let inch_thousandths = decimal_scaled(inch_text, 1_000, "mark")?;
        if inch_thousandths >= 12_000 {
            return Err(DomainError::OutOfRange { field: "mark" });
        }
        let total_thousandths = feet_value
            .checked_mul(12_000)
            .and_then(|value| value.checked_add(inch_thousandths))
            .ok_or(DomainError::OutOfRange { field: "mark" })?;
        return total_thousandths
            .checked_mul(25_400)
            .and_then(|value| positive(value, "mark").ok())
            .map(Nanometers)
            .ok_or(DomainError::OutOfRange { field: "mark" });
    }
    let (number, unit) = split_unit(value);
    let imperial_scale = match unit.to_ascii_lowercase().as_str() {
        "ft" | "feet" | "foot" => Some(304_800),
        "in" | "inch" | "inches" => Some(25_400),
        _ => None,
    };
    if let Some(scale) = imperial_scale {
        return decimal_scaled(number, 1_000, "mark")?
            .checked_mul(scale)
            .and_then(|value| positive(value, "mark").ok())
            .map(Nanometers)
            .ok_or(DomainError::OutOfRange { field: "mark" });
    }
    let scale = match unit.to_ascii_lowercase().as_str() {
        "m" | "meters" | "meter" => 1_000_000_000,
        "cm" | "centimeters" | "centimeter" => 10_000_000,
        "mm" | "millimeters" | "millimeter" => 1_000_000,
        "" => return Err(DomainError::InvalidFormat { field: "mark" }),
        _ => return Err(DomainError::InvalidFormat { field: "mark" }),
    };
    decimal_scaled(number, scale, "mark")
        .and_then(|value| positive(value, "mark"))
        .map(Nanometers)
}

fn split_unit(value: &str) -> (&str, &str) {
    let index = match value.find(|ch: char| ch.is_ascii_alphabetic()) {
        Some(index) => index,
        None => value.len(),
    };
    value.split_at(index)
}

pub(super) fn parse_integer(raw: &str, field: &'static str) -> Result<u64, DomainError> {
    let value = raw.trim();
    if value.starts_with('-') {
        return Err(DomainError::OutOfRange { field });
    }
    if value.is_empty() || !value.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(DomainError::InvalidFormat { field });
    }
    value.chars().try_fold(0u64, |total, ch| {
        let digit = match ch.to_digit(10) {
            Some(digit) => u64::from(digit),
            None => return Err(DomainError::InvalidFormat { field }),
        };
        total
            .checked_mul(10)
            .and_then(|item| item.checked_add(digit))
            .ok_or(DomainError::OutOfRange { field })
    })
}

fn decimal_scaled(raw: &str, scale: u64, field: &'static str) -> Result<u64, DomainError> {
    let value = raw.trim();
    if value.starts_with('-') {
        return Err(DomainError::OutOfRange { field });
    }
    let (whole, fraction) = value.split_once('.').map_or((value, ""), |parts| parts);
    if whole.is_empty()
        || value.ends_with('.')
        || !whole.chars().all(|ch| ch.is_ascii_digit())
        || !fraction.chars().all(|ch| ch.is_ascii_digit())
    {
        return Err(DomainError::InvalidFormat { field });
    }
    let precision =
        usize::try_from(scale.ilog10()).map_err(|_| DomainError::OutOfRange { field })?;
    let excess = fraction.get(precision..).unwrap_or_default();
    if excess.chars().any(|ch| ch != '0') {
        return Err(DomainError::InvalidFormat { field });
    }
    let kept = fraction
        .get(..fraction.len().min(precision))
        .unwrap_or_default();
    let whole_value = parse_integer(whole, field)?
        .checked_mul(scale)
        .ok_or(DomainError::OutOfRange { field })?;
    let fractional_value = if kept.is_empty() {
        0
    } else {
        parse_integer(kept, field)?
            .checked_mul(
                10u64.pow(
                    u32::try_from(precision - kept.len())
                        .map_err(|_| DomainError::OutOfRange { field })?,
                ),
            )
            .ok_or(DomainError::OutOfRange { field })?
    };
    whole_value
        .checked_add(fractional_value)
        .ok_or(DomainError::OutOfRange { field })
}
fn positive(value: u64, field: &'static str) -> Result<u64, DomainError> {
    (value > 0)
        .then_some(value)
        .ok_or(DomainError::OutOfRange { field })
}
