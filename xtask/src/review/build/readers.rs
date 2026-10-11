use anyhow::{Context, Result};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub(super) fn read_json(path: &Path) -> Result<Value> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

pub(super) fn read_optional(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("reading {}", path.display())),
    }
}

pub(super) fn read_optional_json(path: &Path) -> Result<Option<Value>> {
    read_optional(path)?
        .map(|text| {
            serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
        })
        .transpose()
}

pub(super) fn read_kv(path: &Path) -> Result<BTreeMap<String, String>> {
    let Some(text) = read_optional(path)? else {
        return Ok(BTreeMap::new());
    };
    Ok(text
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
        .collect())
}

pub(super) fn read_binaries(path: &Path) -> Result<Map<String, Value>> {
    let Some(text) = read_optional(path)? else {
        return Ok(Map::new());
    };
    let mut out = Map::new();
    for line in text.lines() {
        if let Some((sha, path)) = line.split_once("  ") {
            let name = Path::new(path.trim())
                .file_name()
                .and_then(|name| name.to_str())
                .map_or_else(|| path.trim().to_string(), str::to_string);
            out.insert(name, Value::String(sha.trim().to_string()));
        }
    }
    Ok(out)
}

pub(super) fn read_submission(path: &Path) -> Result<Option<Value>> {
    let Some(text) = read_optional(path)? else {
        return Ok(None);
    };
    let tokens: Vec<&str> = text.split_whitespace().collect();
    match tokens.as_slice() {
        [identity, "invocation", invocation, ..] => Ok(Some(
            json!({"run_identity": identity, "invocation": invocation}),
        )),
        _ => Ok(Some(json!({"raw": text.trim()}))),
    }
}

pub(super) fn read_services(path: &Path) -> Result<u64> {
    let Some(text) = read_optional(path)? else {
        return Ok(0);
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return Ok(0);
    };
    let Some(services) = value.get("services").and_then(Value::as_array) else {
        return Ok(0);
    };
    u64::try_from(services.len()).map_err(|_| anyhow::anyhow!("service count does not fit in u64"))
}

pub(super) fn drain_certificate(log: &str) -> Option<String> {
    log.lines()
        .find_map(|line| line.split("drained: ").nth(1))
        .map(|rest| format!("drained: {rest}"))
}
