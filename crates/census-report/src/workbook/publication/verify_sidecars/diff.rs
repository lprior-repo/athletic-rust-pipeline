use serde_json::Value;

use super::excerpt;

const MAX_STEPS: usize = 4_096;

pub(super) fn difference(expected: &Value, found: &Value) -> String {
    let mut stack = vec![("$".to_string(), expected, found)];
    let mut steps = 0usize;
    while let Some((path, left, right)) = stack.pop() {
        steps = steps.saturating_add(1);
        if steps > MAX_STEPS {
            return format!("{path}: too many differing entries");
        }
        if left == right {
            continue;
        }
        match (left, right) {
            (Value::Object(left_map), Value::Object(right_map)) => {
                if let Some(extra) = right_map.keys().find(|key| !left_map.contains_key(*key)) {
                    return format!("{path}: unexpected key {extra:?}");
                }
                for (key, value) in left_map.iter().rev() {
                    match right_map.get(key) {
                        Some(found) => stack.push((format!("{path}.{key}"), value, found)),
                        None => return format!("{path}.{key}: missing"),
                    }
                }
            }
            (Value::Array(left_rows), Value::Array(right_rows)) => {
                if left_rows.len() != right_rows.len() {
                    return format!(
                        "{path}: expected {} entries, found {}",
                        left_rows.len(),
                        right_rows.len()
                    );
                }
                for (index, (value, found)) in
                    left_rows.iter().zip(right_rows.iter()).enumerate().rev()
                {
                    stack.push((format!("{path}[{index}]"), value, found));
                }
            }
            _ => {
                return format!(
                    "{path}: expected {}, found {}",
                    excerpt(&left.to_string()),
                    excerpt(&right.to_string())
                );
            }
        }
    }
    String::new()
}
