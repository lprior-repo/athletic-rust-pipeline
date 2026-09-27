use serde_json::Value;
use std::collections::BTreeSet;

pub fn ns_key(namespace: &Value) -> String {
    match namespace {
        Value::String(s) => s.clone(),
        Value::Object(map) if !map.is_empty() => {
            if let Some((key, payload)) = map.iter().next() {
                match payload {
                    Value::Object(inner) if !inner.is_empty() => {
                        if let Some(inner_val) = inner.values().next() {
                            match inner_val {
                                Value::String(s) => s.clone(),
                                _ => inner_val.to_string(),
                            }
                        } else {
                            key.clone()
                        }
                    }
                    _ => key.clone(),
                }
            } else {
                "unknown".to_string()
            }
        }
        _ => "unknown".to_string(),
    }
}

pub fn identities(record: &Value) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    if let Some(idents) = record.get("source_identities").and_then(|v| v.as_array()) {
        for identity in idents {
            let ns = identity.get("namespace").unwrap_or(&Value::Null);
            let key = ns_key(ns);
            let id = identity
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let url = identity
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            out.push((key, id, url));
        }
    }
    out
}

pub fn pick(record: &Value, prefix: &str, kind: Option<&str>) -> String {
    for (key, ident, _url) in identities(record) {
        if key == prefix {
            return ident;
        }
        if let Some(k) = kind {
            if key == format!("{prefix}:{k}") {
                return ident;
            }
        }
    }
    String::new()
}

pub fn sources(record: &Value) -> String {
    let mut set = BTreeSet::new();
    if let Some(evidence) = record.get("evidence").and_then(|v| v.as_array()) {
        for e in evidence {
            if let Some(source) = e.get("source") {
                if let Some(id) = source.get("id").and_then(|v| v.as_str()) {
                    set.insert(id.to_string());
                }
            }
        }
    }
    let mut items: Vec<&str> = set.iter().map(|s| s.as_str()).collect();
    items.sort();
    items.join(";")
}

pub fn sports(record: &Value) -> String {
    if let Some(sports) = record.get("sports").and_then(|v| v.as_array()) {
        let items: Vec<String> = sports
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        items.join(";")
    } else {
        String::new()
    }
}
