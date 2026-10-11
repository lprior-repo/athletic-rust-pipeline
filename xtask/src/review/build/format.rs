use serde_json::Value;

pub(super) fn u(value: &Value, key: &str) -> u64 {
    value
        .get(key)
        .and_then(Value::as_u64)
        .map_or(0, core::convert::identity)
}

pub(super) fn s<'a>(value: &'a Value, key: &str) -> &'a str {
    value
        .get(key)
        .and_then(Value::as_str)
        .map_or("", core::convert::identity)
}

pub(super) fn short(text: &str) -> String {
    text.chars().take(16).collect()
}

pub(super) fn list(value: &Value) -> String {
    value
        .as_array()
        .map_or_else(Vec::new, |items| {
            items
                .iter()
                .map(|item| {
                    item.as_str()
                        .map_or_else(|| item.to_string(), str::to_string)
                })
                .collect()
        })
        .join(", ")
}

pub(super) fn object_entries<F: Fn(&str) -> String>(value: &Value, render: F) -> String {
    value
        .as_object()
        .map_or_else(Vec::new, |map| {
            map.iter()
                .map(|(key, value)| {
                    let shown = value.as_str().map_or_else(String::new, str::to_string);
                    format!("{key} {}", render(&shown))
                })
                .collect()
        })
        .join(", ")
}

pub(super) fn dash(value: &Value) -> String {
    value
        .as_str()
        .map_or_else(|| "none recorded".to_string(), str::to_string)
}
