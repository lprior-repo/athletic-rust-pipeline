//! Centimetres (hundredths of a metre).

use serde::{de, Deserialize, Deserializer, Serialize};

/// Centimetres (hundredths of a metre).  Range: ±21 474 836m.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Default)]
#[serde(transparent)]
pub struct CentiMetres(i32);

impl CentiMetres {
    /// Wrap an exact centimetre count.
    pub const fn new(centimetres: i32) -> Self {
        Self(centimetres)
    }

    /// Scale a value already known to be in metres into centimetres, refusing NaN, infinity and
    /// anything that would not fit in `i32`.
    pub fn try_from_metres_f64(v: f64) -> Option<Self> {
        super::checked_hundredths(v).map(Self)
    }

    /// The exact centimetre count.
    pub const fn value(self) -> i32 {
        self.0
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
                match super::checked_hundredths(v) {
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
