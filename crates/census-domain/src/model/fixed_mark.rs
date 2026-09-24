//! Fixed-point wrappers for the numeric channels inside [`Mark`].
//!
//! All three store an exact integer in the source's native sub-unit, so equality and ordering are
//! integer comparisons — no floating-point rounding surprises.
//!
//! Wire form is unchanged: serde serialises them as the human-readable decimal (e.g. `10.94`)
//! rather than the raw integer, so existing persisted JSON is byte-identical.

use serde::{Deserialize, Serialize};

/// Centiseconds (hundredths of a second).  Range: ±214 748 364s (~60h).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(transparent)]
pub struct CentiSeconds(pub i32);

impl CentiSeconds {
    /// Parse a decimal string like `"10.94"` or `"4:41.23"` (already in seconds).
    pub fn from_seconds_f64(v: f64) -> Self {
        Self((v * 100.0).round() as i32)
    }

    /// Convert the stored integer back to seconds as an f64 — lossless for values ≤ 999 999.
    pub fn as_seconds_f64(self) -> f64 {
        self.0 as f64 / 100.0
    }
}

impl std::fmt::Display for CentiSeconds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{:02}", self.0 / 100, (self.0 % 100).abs())
    }
}

/// Centimetres (hundredths of a metre).  Range: ±21 474 836m.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(transparent)]
pub struct CentiMetres(pub i32);

impl CentiMetres {
    /// Parse a decimal metre string like `"12.34"`.
    pub fn from_metres_f64(v: f64) -> Self {
        Self((v * 100.0).round() as i32)
    }

    /// Convert the stored integer back to metres as an f64.
    pub fn as_metres_f64(self) -> f64 {
        self.0 as f64 / 100.0
    }
}

impl std::fmt::Display for CentiMetres {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{:02}", self.0 / 100, (self.0 % 100).abs())
    }
}

/// Centi-points (hundredths of a point).  Range: ±21 474 836pts.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[serde(transparent)]
pub struct CentiPoints(pub i32);

impl CentiPoints {
    /// Parse a decimal string like `"3456"`.
    pub fn from_points_f64(v: f64) -> Self {
        Self((v * 100.0).round() as i32)
    }

    /// Convert the stored integer back to points as an f64.
    pub fn as_points_f64(self) -> f64 {
        self.0 as f64 / 100.0
    }
}

#[cfg(test)]
#[path = "fixed_mark_tests.rs"]
mod tests;
