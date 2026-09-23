//! Number helpers shared by the scans and the debt ratchet.
//!
//! Every count this crate emits is a JSON integer, and every number it reads back comes from a file
//! `xtask` itself wrote. These helpers keep the conversions total: a missing key, a wrong type or a
//! value that does not fit reads as `0` (Python's `.get(key, 0)`), and a `usize` count saturates
//! instead of wrapping. Nothing here can panic.

use serde_json::Value;

/// A `usize` count as a JSON integer.
///
/// Infallible on 64-bit targets; saturation keeps it total everywhere else.
pub fn count(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

/// A JSON integer, or `0` when the key is absent or is not a non-negative integer.
pub fn number(value: Option<&Value>) -> u64 {
    value.and_then(Value::as_u64).unwrap_or(0)
}

/// Python truthiness, for the one place the deleted scripts branched on it (`if not allow and old`).
pub fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(flag) => *flag,
        Value::Number(number) => number.as_f64().is_some_and(|value| value != 0.0),
        Value::String(text) => !text.is_empty(),
        Value::Array(items) => !items.is_empty(),
        Value::Object(entries) => !entries.is_empty(),
    }
}
