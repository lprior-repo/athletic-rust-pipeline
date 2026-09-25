//! Centiseconds (hundredths of a second).

use serde::{de, Deserialize, Deserializer, Serialize};

/// Centiseconds (hundredths of a second).  Range: ±214 748 364s (~60h).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Default)]
#[serde(transparent)]
pub struct CentiSeconds(i32);

impl CentiSeconds {
    /// Wrap an exact centisecond count. Every `i32` is a valid count, so this cannot fail; the
    /// guard that matters sits on the float path below, which has to settle a unit before scaling.
    pub const fn new(centiseconds: i32) -> Self {
        Self(centiseconds)
    }

    /// Scale a value already known to be in seconds into centiseconds, refusing NaN, infinity and
    /// anything that would not fit in `i32`.
    ///
    /// Callers own their source format, so they own the refusal: nothing here substitutes `0` for
    /// NaN or pins a bound for an overflow, because either would publish a mark the source never
    /// stated.
    pub fn try_from_seconds_f64(v: f64) -> Option<Self> {
        super::checked_hundredths(v).map(Self)
    }

    /// The exact centisecond count.
    pub const fn value(self) -> i32 {
        self.0
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
                match super::checked_hundredths(v) {
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
