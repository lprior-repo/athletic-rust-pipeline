use serde_json::Value;

use super::object_wire::JournalPath;
use crate::journal::{
    decode_chunk_into, CHUNK_TYPE, JOURNAL_CHUNK_RAW, JOURNAL_CHUNK_VERSION, MANIFEST_TYPE,
};
use crate::{JournalReadBudget, StoreError, StoreResult, MAX_JOURNAL_VALUE_BYTES};

pub(super) enum LegacyRoot {
    Inline(Value),
    Chunked(LegacyBounds),
}

pub(super) struct LegacyBounds {
    pub(super) chunks: usize,
    pub(super) total_bytes: usize,
}

impl LegacyBounds {
    pub(super) fn range(&self, index: usize) -> Option<std::ops::Range<usize>> {
        if index >= self.chunks {
            return None;
        }
        let start = index.checked_mul(JOURNAL_CHUNK_RAW)?;
        let end = start.checked_add(JOURNAL_CHUNK_RAW)?.min(self.total_bytes);
        (start < end).then_some(start..end)
    }
}

pub(super) fn root(
    path: JournalPath<'_>,
    raw: &[u8],
    budget: JournalReadBudget,
) -> StoreResult<LegacyRoot> {
    limit(path, "physical root", raw.len(), MAX_JOURNAL_VALUE_BYTES)?;
    let value = record(path, "root", raw)?;
    match value.get("type") {
        None => {
            limit(
                path,
                "encoded envelope",
                raw.len(),
                budget.max_encoded_bytes(),
            )?;
            payload(path, value).map(LegacyRoot::Inline)
        }
        Some(kind) if kind.as_str() == Some(MANIFEST_TYPE) => {
            bounds(path, &value, budget).map(LegacyRoot::Chunked)
        }
        Some(_) => Err(path.refusal("root is neither an inline entry nor a chunked-manifest")),
    }
}

fn bounds(
    path: JournalPath<'_>,
    value: &Value,
    budget: JournalReadBudget,
) -> StoreResult<LegacyBounds> {
    version(path, "manifest", value)?;
    if value.get("key").and_then(Value::as_str) != Some(path.key) {
        return Err(path.refusal("manifest key does not match its lookup"));
    }
    let total_bytes = declared(path, "manifest", value, "total_bytes")?;
    limit(
        path,
        "encoded envelope",
        total_bytes,
        budget.max_encoded_bytes(),
    )?;
    let expected = total_bytes
        .checked_sub(1)
        .and_then(|bytes| (bytes / JOURNAL_CHUNK_RAW).checked_add(1))
        .ok_or_else(|| path.refusal("manifest has no encoded envelope bytes"))?;
    let chunks = declared(path, "manifest", value, "chunks")?;
    if chunks != expected {
        return Err(path.refusal("manifest chunk count does not cover its byte length"));
    }
    Ok(LegacyBounds {
        chunks,
        total_bytes,
    })
}

pub(super) fn chunk_into(
    path: JournalPath<'_>,
    bounds: &LegacyBounds,
    index: usize,
    raw: &[u8],
    output: &mut [u8],
) -> StoreResult<()> {
    limit(path, "physical chunk", raw.len(), MAX_JOURNAL_VALUE_BYTES)?;
    let what = format!("chunk {index}");
    let value = record(path, &what, raw)?;
    chunk_owner(path, bounds, index, &value)?;
    let data = value
        .get("data")
        .and_then(Value::as_str)
        .ok_or_else(|| path.refusal(&format!("{what} has no base64 data")))?;
    let ceiling = JOURNAL_CHUNK_RAW
        .checked_add(2)
        .and_then(|bytes| (bytes / 3).checked_mul(4))
        .ok_or_else(|| path.refusal("chunk codec ceiling cannot be represented"))?;
    limit(path, "chunk base64", data.len(), ceiling)?;
    let decoded_bytes = decoded_size(path, &what, data)?;
    limit(path, "chunk decoded", decoded_bytes, JOURNAL_CHUNK_RAW)?;
    if decoded_bytes != output.len() {
        return Err(path.refusal(&format!(
            "{what} decoded length does not match its manifest"
        )));
    }
    let decoded = decode_chunk_into(data, output).map_err(|source| match source {
        base64::DecodeSliceError::OutputSliceTooSmall => path.refusal(&format!(
            "{what} decoded length does not match its manifest"
        )),
        base64::DecodeSliceError::DecodeError(_) => {
            path.refusal(&format!("{what} data is not valid base64"))
        }
    })?;
    if decoded != output.len() {
        return Err(path.refusal(&format!(
            "{what} decoded length does not match its manifest"
        )));
    }
    Ok(())
}

fn decoded_size(path: JournalPath<'_>, what: &str, data: &str) -> StoreResult<usize> {
    if !data.len().is_multiple_of(4) {
        return Err(path.refusal(&format!("{what} data is not valid base64")));
    }
    let padding = if data.ends_with("==") {
        2
    } else if data.ends_with('=') {
        1
    } else {
        0
    };
    (data.len() / 4)
        .checked_mul(3)
        .and_then(|bytes| bytes.checked_sub(padding))
        .ok_or_else(|| path.refusal(&format!("{what} data is not valid base64")))
}

fn chunk_owner(
    path: JournalPath<'_>,
    bounds: &LegacyBounds,
    index: usize,
    value: &Value,
) -> StoreResult<()> {
    let what = format!("chunk {index}");
    if value.get("type").and_then(Value::as_str) != Some(CHUNK_TYPE) {
        return Err(path.refusal(&format!("{what} is not a chunk envelope")));
    }
    version(path, &what, value)?;
    if value.get("key").and_then(Value::as_str) != Some(path.key) {
        return Err(path.refusal(&format!("{what} key does not match its lookup")));
    }
    if declared(path, &what, value, "index")? != index {
        return Err(path.refusal(&format!("{what} ordinal does not match its key")));
    }
    if declared(path, &what, value, "total")? != bounds.chunks {
        return Err(path.refusal(&format!("{what} total does not match its manifest")));
    }
    Ok(())
}

pub(super) fn payload(path: JournalPath<'_>, mut value: Value) -> StoreResult<Value> {
    if value.get("type").is_some() || value.get("version").is_some() {
        return Err(path.refusal("entry contains an unsupported type or version"));
    }
    if value.get("key").and_then(Value::as_str) != Some(path.key) {
        return Err(path.refusal("entry key does not match its lookup"));
    }
    value
        .as_object_mut()
        .and_then(|entry| entry.remove("payload"))
        .ok_or_else(|| path.refusal("entry has no payload"))
}

pub(super) fn record(path: JournalPath<'_>, what: &str, raw: &[u8]) -> StoreResult<Value> {
    serde_json::from_slice(raw).map_err(|source| StoreError::Decode {
        key: format!("legacy journal {what} for {}/{}", path.phase, path.key),
        source,
    })
}

fn version(path: JournalPath<'_>, what: &str, value: &Value) -> StoreResult<()> {
    if value.get("version").and_then(Value::as_u64) == Some(u64::from(JOURNAL_CHUNK_VERSION)) {
        return Ok(());
    }
    Err(path.refusal(&format!("{what} is not version {JOURNAL_CHUNK_VERSION}")))
}

fn declared(path: JournalPath<'_>, what: &str, value: &Value, field: &str) -> StoreResult<usize> {
    let number = value
        .get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| path.refusal(&format!("{what} has no unsigned {field}")))?;
    usize::try_from(number)
        .map_err(|_| path.refusal(&format!("{what} {field} cannot be represented")))
}

pub(super) fn limit(
    path: JournalPath<'_>,
    what: &'static str,
    needed: usize,
    max: usize,
) -> StoreResult<()> {
    if needed <= max {
        return Ok(());
    }
    Err(StoreError::JournalReadLimit {
        phase: path.phase.to_string(),
        key: path.key.to_string(),
        what,
        needed,
        max,
    })
}
