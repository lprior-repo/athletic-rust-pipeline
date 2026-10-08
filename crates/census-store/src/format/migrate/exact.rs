use census_domain::model::ExactSeconds;
use serde::Deserialize;
use serde_json::Value;

use crate::{StoreError, StoreResult};

pub(super) fn convert_mark(mark: &mut Value) -> StoreResult<bool> {
    if mark.get("TimeSeconds").is_none() {
        return Ok(false);
    }
    if mark.as_object().map(serde_json::Map::len) != Some(1) {
        return Err(invalid(
            "canonical time mark must contain exactly one variant",
        ));
    }
    let time = mark
        .get_mut("TimeSeconds")
        .ok_or_else(|| invalid("canonical time mark missing"))?;
    if time.is_object() {
        ExactSeconds::deserialize(&*time).map_err(|source| StoreError::Json {
            detail: "invalid migrated exact time".into(),
            source,
        })?;
        return Ok(false);
    }
    let centiseconds = time
        .as_i64()
        .ok_or_else(|| invalid("legacy time is not an integer"))?;
    let nanoseconds = centiseconds
        .checked_mul(10_000_000)
        .ok_or_else(|| invalid("legacy centiseconds overflow the nanosecond range"))?;
    let exact =
        ExactSeconds::from_parts(nanoseconds, 2).map_err(|error| invalid(&error.to_string()))?;
    *time = serde_json::to_value(exact).map_err(|source| StoreError::Json {
        detail: "encoding migrated exact time".into(),
        source,
    })?;
    Ok(true)
}

pub(super) fn convert_performance(key: &[u8], bytes: &[u8]) -> StoreResult<Option<Vec<u8>>> {
    let mut row: Value = decode(bytes)?;
    let (_, id, _) = crate::keys::view_observation_key(key)
        .ok_or_else(|| invalid("performance has a malformed observation key"))?;
    if id.is_empty() || row.get("id").and_then(Value::as_str).map(str::as_bytes) != Some(id) {
        return Err(invalid(
            "performance id does not match its durable observation key",
        ));
    }
    let mark = row
        .get_mut("mark")
        .ok_or_else(|| invalid("performance has no mark"))?;
    if !convert_mark(mark)? {
        return Ok(None);
    }
    encode(&row).map(Some)
}

pub(super) fn decode(bytes: &[u8]) -> StoreResult<Value> {
    serde_json::from_slice(bytes).map_err(|source| StoreError::Json {
        detail: "decoding schema migration row".into(),
        source,
    })
}

pub(super) fn encode(row: &Value) -> StoreResult<Vec<u8>> {
    serde_json::to_vec(row).map_err(|source| StoreError::Json {
        detail: "encoding schema migration row".into(),
        source,
    })
}

pub(super) fn invalid(detail: &str) -> StoreError {
    StoreError::Refused {
        detail: format!("exact-time migration unfinished: {detail}"),
    }
}
