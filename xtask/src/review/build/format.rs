use serde_json::{json, Value};

pub(super) fn u_opt(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}

pub(super) fn number(value: Option<u64>) -> Value {
    value.map_or(Value::Null, |count| json!(count))
}

pub(super) fn count(value: &Value, key: &str) -> Value {
    number(u_opt(value, key))
}

pub(super) fn tally(value: &Value) -> Option<u64> {
    value
        .as_array()
        .and_then(|items| u64::try_from(items.len()).ok())
}

pub(super) fn shown(value: Option<u64>) -> String {
    value.map_or_else(|| "?".to_string(), |count| count.to_string())
}

pub(super) fn display(value: Option<&Value>) -> String {
    value
        .filter(|value| !value.is_null())
        .map_or_else(|| "?".to_string(), Value::to_string)
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

#[cfg(test)]
mod tests {
    use super::{count, number, shown};
    use serde_json::{json, Value};

    #[test]
    fn absent_or_non_numeric_counts_read_as_null() {
        assert_eq!(count(&json!({"rows": 3}), "rows"), json!(3));
        assert_eq!(count(&json!({"rows": "3"}), "rows"), Value::Null);
        assert_eq!(count(&json!({}), "rows"), Value::Null);
    }

    #[test]
    fn nullable_counts_render_as_a_question_mark() {
        assert_eq!(number(None), Value::Null);
        assert_eq!(number(Some(0)), json!(0));
        assert_eq!(shown(None), "?");
        assert_eq!(shown(Some(0)), "0");
    }
}
