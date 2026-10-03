use serde::{Deserialize, Serialize};
use std::fmt;

use super::DirectoryError;

pub const SCALE: u32 = 10_000_000;

const LATITUDE_MAX: i64 = 900_000_000;

const LONGITUDE_MAX: i64 = 1_800_000_000;

const FRACTION_DIGITS: usize = 7;

const COORDINATE_TEXT_LIMIT: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "i32")]
pub struct Latitude(i32);

impl Latitude {
    pub fn from_decimal_text(raw: &str) -> Result<Self, DirectoryError> {
        scaled(raw, "latitude", LATITUDE_MAX).map(Self)
    }

    pub fn scaled(self) -> i32 {
        self.0
    }
}

impl TryFrom<i32> for Latitude {
    type Error = DirectoryError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        if i64::from(value).abs() > LATITUDE_MAX {
            return Err(DirectoryError::CoordinateOutOfRange {
                field: "latitude",
                value: i64::from(value),
            });
        }
        Ok(Self(value))
    }
}

impl fmt::Display for Latitude {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_scaled(formatter, self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "i32")]
pub struct Longitude(i32);

impl Longitude {
    pub fn from_decimal_text(raw: &str) -> Result<Self, DirectoryError> {
        scaled(raw, "longitude", LONGITUDE_MAX).map(Self)
    }

    pub fn scaled(self) -> i32 {
        self.0
    }
}

impl TryFrom<i32> for Longitude {
    type Error = DirectoryError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        if i64::from(value).abs() > LONGITUDE_MAX {
            return Err(DirectoryError::CoordinateOutOfRange {
                field: "longitude",
                value: i64::from(value),
            });
        }
        Ok(Self(value))
    }
}

impl fmt::Display for Longitude {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_scaled(formatter, self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Coordinates {
    latitude: Latitude,
    longitude: Longitude,
}

impl Coordinates {
    pub fn parse(latitude: &str, longitude: &str) -> Result<Self, DirectoryError> {
        Ok(Self {
            latitude: Latitude::from_decimal_text(latitude)?,
            longitude: Longitude::from_decimal_text(longitude)?,
        })
    }

    pub fn latitude(self) -> Latitude {
        self.latitude
    }

    pub fn longitude(self) -> Longitude {
        self.longitude
    }
}

fn write_scaled(formatter: &mut fmt::Formatter<'_>, scaled: i32) -> fmt::Result {
    if scaled < 0 {
        formatter.write_str("-")?;
    }
    let magnitude = scaled.unsigned_abs();
    let degrees = magnitude / SCALE;
    let fraction = magnitude % SCALE;
    write!(formatter, "{degrees}")?;
    if fraction > 0 {
        let text = format!("{fraction:07}");
        write!(formatter, ".{}", text.trim_end_matches('0'))?;
    }
    Ok(())
}

fn scaled(raw: &str, field: &'static str, maximum: i64) -> Result<i32, DirectoryError> {
    let malformed = || DirectoryError::MalformedCoordinate {
        value: raw.to_string(),
    };
    let trimmed = raw.trim();
    if trimmed.len() > COORDINATE_TEXT_LIMIT {
        return Err(DirectoryError::FieldTooLong {
            field,
            limit: COORDINATE_TEXT_LIMIT,
            length: trimmed.len(),
        });
    }
    let value = parse_scaled(trimmed).ok_or_else(malformed)?;
    if value.abs() > maximum {
        return Err(DirectoryError::CoordinateOutOfRange { field, value });
    }
    i32::try_from(value).map_err(|_| DirectoryError::CoordinateOutOfRange { field, value })
}

fn parse_scaled(trimmed: &str) -> Option<i64> {
    let (negative, body) = match trimmed.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (
            false,
            trimmed.strip_prefix('+').map_or(trimmed, |value| value),
        ),
    };
    let mut segments = body.split('.');
    let whole = segments.next().map_or(Default::default(), core::convert::identity);
    let fraction = segments.next().map_or(Default::default(), core::convert::identity);
    if segments.next().is_some() || (whole.is_empty() && fraction.is_empty()) {
        return None;
    }
    if !whole.chars().all(|ch| ch.is_ascii_digit())
        || !fraction.chars().all(|ch| ch.is_ascii_digit())
    {
        return None;
    }
    let degrees: i64 = whole.parse().ok()?;
    let mut units: i64 = 0;
    let mut round_up = false;
    let mut kept = 0usize;
    for (index, ch) in fraction.chars().enumerate() {
        let digit = i64::from(ch.to_digit(10).map_or(Default::default(), core::convert::identity));
        if index < FRACTION_DIGITS {
            units = units
                .checked_mul(10)
                .and_then(|value| value.checked_add(digit))?;
            kept = kept.saturating_add(1);
        } else if index == FRACTION_DIGITS {
            round_up = digit >= 5;
        }
    }
    let mut padded = units;
    for _ in kept..FRACTION_DIGITS {
        padded = padded.checked_mul(10)?;
    }
    let mut value = degrees
        .checked_mul(i64::from(SCALE))
        .and_then(|value| value.checked_add(padded))?;
    if round_up {
        value = value.checked_add(1)?;
    }
    if negative {
        value = value.checked_neg()?;
    }
    Some(value)
}
