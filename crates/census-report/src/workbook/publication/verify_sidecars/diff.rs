use serde_json::{Map, Value};

use super::excerpt;

const MAX_STEPS: usize = 4_096;
type Pending<'a> = Vec<(String, &'a Value, &'a Value)>;

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
        if let Some(difference) = inspect(&path, left, right, &mut stack) {
            return difference;
        }
    }
    String::new()
}

fn inspect<'a>(
    path: &str,
    left: &'a Value,
    right: &'a Value,
    stack: &mut Pending<'a>,
) -> Option<String> {
    match (left, right) {
        (Value::Object(left), Value::Object(right)) => objects(path, left, right, stack),
        (Value::Array(left), Value::Array(right)) => arrays(path, left, right, stack),
        _ => Some(format!(
            "{path}: expected {}, found {}",
            excerpt(&left.to_string()),
            excerpt(&right.to_string())
        )),
    }
}

fn objects<'a>(
    path: &str,
    left: &'a Map<String, Value>,
    right: &'a Map<String, Value>,
    stack: &mut Pending<'a>,
) -> Option<String> {
    if let Some(extra) = right.keys().find(|key| !left.contains_key(*key)) {
        return Some(format!("{path}: unexpected key {extra:?}"));
    }
    for (key, value) in left.iter().rev() {
        match right.get(key) {
            Some(found) => stack.push((format!("{path}.{key}"), value, found)),
            None => return Some(format!("{path}.{key}: missing")),
        }
    }
    None
}

fn arrays<'a>(
    path: &str,
    left: &'a [Value],
    right: &'a [Value],
    stack: &mut Pending<'a>,
) -> Option<String> {
    if left.len() != right.len() {
        return Some(format!(
            "{path}: expected {} entries, found {}",
            left.len(),
            right.len()
        ));
    }
    for (index, (value, found)) in left.iter().zip(right.iter()).enumerate().rev() {
        stack.push((format!("{path}[{index}]"), value, found));
    }
    None
}
