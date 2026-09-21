use serde::{Deserialize, Serialize};

/// Validated event metadata from GetNavInfo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NavEvent {
    pub id: u64,
    pub name: String,
    pub t: String,
    pub m: String,
    pub r: bool,
    pub h: bool,
    pub short: String,
    pub w: bool,
    pub fmt: Option<String>,
    pub so: u64,
}

/// Excluded event patterns: walk/race-walk only.
pub fn is_excluded(short: &str, _r: bool, _h: bool) -> bool {
    short
        .as_bytes()
        .windows(4)
        .any(|window| window.eq_ignore_ascii_case(b"walk"))
}

impl NavEvent {
    /// Parse a NavEvent from a borrowed JSON value without cloning the value.
    pub fn from_value(value: &serde_json::Value) -> Option<Self> {
        Self::deserialize(value).ok()
    }
}
