//! Fixed-point wrappers for the numeric channels inside [`Mark`].
//!
//! All three store an exact integer in the source's native sub-unit, so equality and ordering are
//! integer comparisons — no floating-point rounding surprises.
//!
//! Wire form on the current writer is a raw integer (e.g. `1094` for 10.94 s). The reader also
//! accepts the legacy float the old writer emitted (e.g. `10.94`), converting with the same
//! `round(x * 100)` rounding the constructors use, and fails closed if the scaled value does not
//! fit in `i32`. This dual-read path exists solely because rows persisted before the migration
//! carry the float form and the append-only store cannot be rewritten.

use serde::{de, Deserialize, Deserializer, Serialize};

/// Scale a decimal source value to integer hundredths, refusing anything that would not fit.
///
/// std has no checked conversion from a float to an integer, so the range test below is what lets
/// the one cast here be provably in range. It is the module's only cast, and it lives here so the
/// tree's `as_cast` budget has one documented site to read instead of fifteen scattered ones.
#[allow(clippy::as_conversions)]
fn checked_hundredths(value: f64) -> Option<i32> {
    let scaled = (value * 100.0).round();
    if !scaled.is_finite() || scaled < f64::from(i32::MIN) || scaled > f64::from(i32::MAX) {
        return None;
    }
    Some(scaled as i32)
}

/// [`checked_hundredths`] for the parsers, which hand over a value they have already range-checked
/// and would not want to thread a `Result` through their own callers for.
///
/// The refusing cases cannot come from those callers, so they fall back to what the deleted cast
/// did: NaN reads as zero, and an overflow pins to the bound it ran past.
fn hundredths(value: f64) -> i32 {
    if value.is_nan() {
        return 0;
    }
    checked_hundredths(value).unwrap_or(if value < 0.0 { i32::MIN } else { i32::MAX })
}

/// Centiseconds (hundredths of a second).  Range: ±214 748 364s (~60h).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Default)]
#[serde(transparent)]
pub struct CentiSeconds(pub i32);

impl CentiSeconds {
    /// Parse a decimal string like `"10.94"` or `"4:41.23"` (already in seconds).
    pub fn from_seconds_f64(v: f64) -> Self {
        Self(hundredths(v))
    }

    /// Convert the stored integer back to seconds as an f64 — lossless for values ≤ 999 999.
    pub fn as_seconds_f64(self) -> f64 {
        f64::from(self.0) / 100.0
    }
}

impl<'de> Deserialize<'de> for CentiSeconds {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = CentiSeconds;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(
                    formatter,
                    "an integer or a fractional number of centiseconds"
                )
            }

            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(CentiSeconds(i32::try_from(v).map_err(|_| {
                    E::custom(format_args!("centiseconds value {v} exceeds i32 range"))
                })?))
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(CentiSeconds(i32::try_from(v).map_err(|_| {
                    E::custom(format_args!("centiseconds value {v} exceeds i32 range"))
                })?))
            }

            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
                match checked_hundredths(v) {
                    Some(n) => Ok(CentiSeconds(n)),
                    None => Err(E::custom(format_args!(
                        "centiseconds legacy float {v} overflows i32 after scaling"
                    ))),
                }
            }

            fn visit_i32<E: de::Error>(self, v: i32) -> Result<Self::Value, E> {
                Ok(CentiSeconds(v))
            }

            fn visit_u32<E: de::Error>(self, v: u32) -> Result<Self::Value, E> {
                Ok(CentiSeconds(i32::try_from(v).map_err(|_| {
                    E::custom(format_args!("centiseconds value {v} exceeds i32 range"))
                })?))
            }

            fn visit_f32<E: de::Error>(self, v: f32) -> Result<Self::Value, E> {
                self.visit_f64(f64::from(v))
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

impl std::fmt::Display for CentiSeconds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{:02}", self.0 / 100, (self.0 % 100).abs())
    }
}

/// Centimetres (hundredths of a metre).  Range: ±21 474 836m.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Default)]
#[serde(transparent)]
pub struct CentiMetres(pub i32);

impl CentiMetres {
    /// Parse a decimal metre string like `"12.34"`.
    pub fn from_metres_f64(v: f64) -> Self {
        Self(hundredths(v))
    }

    /// Convert the stored integer back to metres as an f64.
    pub fn as_metres_f64(self) -> f64 {
        f64::from(self.0) / 100.0
    }
}

impl<'de> Deserialize<'de> for CentiMetres {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = CentiMetres;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(
                    formatter,
                    "an integer or a fractional number of centimetres"
                )
            }

            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(CentiMetres(i32::try_from(v).map_err(|_| {
                    E::custom(format_args!("centimetres value {v} exceeds i32 range"))
                })?))
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(CentiMetres(i32::try_from(v).map_err(|_| {
                    E::custom(format_args!("centimetres value {v} exceeds i32 range"))
                })?))
            }

            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
                match checked_hundredths(v) {
                    Some(n) => Ok(CentiMetres(n)),
                    None => Err(E::custom(format_args!(
                        "centimetres legacy float {v} overflows i32 after scaling"
                    ))),
                }
            }

            fn visit_i32<E: de::Error>(self, v: i32) -> Result<Self::Value, E> {
                Ok(CentiMetres(v))
            }

            fn visit_u32<E: de::Error>(self, v: u32) -> Result<Self::Value, E> {
                Ok(CentiMetres(i32::try_from(v).map_err(|_| {
                    E::custom(format_args!("centimetres value {v} exceeds i32 range"))
                })?))
            }

            fn visit_f32<E: de::Error>(self, v: f32) -> Result<Self::Value, E> {
                self.visit_f64(f64::from(v))
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

impl std::fmt::Display for CentiMetres {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{:02}", self.0 / 100, (self.0 % 100).abs())
    }
}

/// Centi-points (hundredths of a point).  Range: ±21 474 836pts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Default)]
#[serde(transparent)]
pub struct CentiPoints(pub i32);

impl CentiPoints {
    /// Parse a decimal string like `"3456"`.
    pub fn from_points_f64(v: f64) -> Self {
        Self(hundredths(v))
    }

    /// Convert the stored integer back to points as an f64.
    pub fn as_points_f64(self) -> f64 {
        f64::from(self.0) / 100.0
    }
}

impl<'de> Deserialize<'de> for CentiPoints {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = CentiPoints;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(
                    formatter,
                    "an integer or a fractional number of centi-points"
                )
            }

            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(CentiPoints(i32::try_from(v).map_err(|_| {
                    E::custom(format_args!("centi-points value {v} exceeds i32 range"))
                })?))
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(CentiPoints(i32::try_from(v).map_err(|_| {
                    E::custom(format_args!("centi-points value {v} exceeds i32 range"))
                })?))
            }

            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
                match checked_hundredths(v) {
                    Some(n) => Ok(CentiPoints(n)),
                    None => Err(E::custom(format_args!(
                        "centi-points legacy float {v} overflows i32 after scaling"
                    ))),
                }
            }

            fn visit_i32<E: de::Error>(self, v: i32) -> Result<Self::Value, E> {
                Ok(CentiPoints(v))
            }

            fn visit_u32<E: de::Error>(self, v: u32) -> Result<Self::Value, E> {
                Ok(CentiPoints(i32::try_from(v).map_err(|_| {
                    E::custom(format_args!("centi-points value {v} exceeds i32 range"))
                })?))
            }

            fn visit_f32<E: de::Error>(self, v: f32) -> Result<Self::Value, E> {
                self.visit_f64(f64::from(v))
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

#[cfg(test)]
#[path = "fixed_mark_tests.rs"]
mod tests;
