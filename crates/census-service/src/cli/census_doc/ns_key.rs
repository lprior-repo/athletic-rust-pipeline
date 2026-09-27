use serde_json::Value;

pub(super) fn ns_key(namespace: &Value) -> String {
    match namespace {
        Value::String(s) => s.clone(),
        Value::Object(map) if !map.is_empty() => match map.iter().next() {
            Some((key, payload)) => {
                if let Value::Object(inner) = payload {
                    if let Some(inner_val) = inner.values().next() {
                        return format!("{key}:{inner_val}");
                    }
                }
                key.clone()
            }
            None => "unknown".to_string(),
        },
        _ => "unknown".to_string(),
    }
}
