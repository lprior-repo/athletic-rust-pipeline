//! The Python `repr` shapes this report prints.
//!
//! The inventory's output was byte-compared against the deleted `inventory-search-pages.py`, so the
//! dict, string and tuple formatting here is a compatibility surface rather than a style choice.

use serde_json::Value;

/// `{'key': value, ...}`, the dict repr the deleted script printed for its tallies.
pub(crate) fn fmt_dict<K: AsRef<str>, V: std::fmt::Display>(items: &[(K, V)]) -> String {
    let parts: Vec<String> = items
        .iter()
        .map(|(k, v)| format!("'{}': {}", k.as_ref(), v))
        .collect();
    format!("{{{}}}", parts.join(", "))
}

/// A JSON boolean as the Python name the report prints for it; anything else is `None`.
pub(crate) fn json_bool_to_str(v: &Value) -> &'static str {
    match v {
        Value::Bool(true) => "True",
        Value::Bool(false) => "False",
        _ => "None",
    }
}

/// A Python string repr: single-quoted, switching to double quotes when the text carries one.
pub(crate) fn py_repr_str(s: &str) -> String {
    if s.contains('\'') {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'"))
    }
}

/// One element of a printed tuple: a number stays bare, everything else is quoted.
pub(crate) fn py_repr_value(v: &str, is_int: bool) -> String {
    if is_int {
        v.to_string()
    } else {
        py_repr_str(v)
    }
}

/// `(a, b, ...)`, with `is_int` marking the elements that print unquoted.
pub(crate) fn py_repr_tuple(elements: &[(&str, bool)]) -> String {
    let parts: Vec<String> = elements
        .iter()
        .map(|(s, is_int)| py_repr_value(s, *is_int))
        .collect();
    format!("({})", parts.join(", "))
}
