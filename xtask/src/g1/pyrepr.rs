use serde_json::Value;

pub(crate) fn fmt_dict<K: AsRef<str>, V: std::fmt::Display>(items: &[(K, V)]) -> String {
    let parts: Vec<String> = items
        .iter()
        .map(|(k, v)| format!("'{}': {}", k.as_ref(), v))
        .collect();
    format!("{{{}}}", parts.join(", "))
}

pub(crate) fn json_bool_to_str(v: &Value) -> &'static str {
    match v {
        Value::Bool(true) => "True",
        Value::Bool(false) => "False",
        _ => "None",
    }
}

pub(crate) fn py_repr_str(s: &str) -> String {
    if s.contains('\'') {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'"))
    }
}

pub(crate) fn py_repr_value(v: &str, is_int: bool) -> String {
    if is_int {
        v.to_string()
    } else {
        py_repr_str(v)
    }
}

pub(crate) fn py_repr_tuple(elements: &[(&str, bool)]) -> String {
    let parts: Vec<String> = elements
        .iter()
        .map(|(s, is_int)| py_repr_value(s, *is_int))
        .collect();
    format!("({})", parts.join(", "))
}
