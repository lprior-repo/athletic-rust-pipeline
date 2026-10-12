use serde_json::Value;

use crate::journal::{
    decode_object_chunk, expected_chunk_count, expected_index_pages, is_object_id, object_digest,
    JOURNAL_OBJECT_VERSION, OBJECT_ALIAS_TYPE, OBJECT_BYTES_LIMIT, OBJECT_CHUNKS_LIMIT,
    OBJECT_CHUNK_TYPE, OBJECT_CODEC_JSONL, OBJECT_INDEX_TYPE, OBJECT_MANIFEST_TYPE,
    OBJECT_RECORDS_LIMIT, OBJECT_STATE_READY,
};
use crate::{StoreError, StoreResult};

pub(super) struct ObjectBounds {
    pub(super) total_bytes: usize,
    pub(super) chunks: usize,
    pub(super) records: usize,
    pub(super) index_pages: usize,
    pub(super) chunk_digests: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RecordRange {
    pub(super) offset: usize,
    pub(super) length: usize,
}

#[derive(Clone, Copy)]
pub(super) struct JournalPath<'a> {
    pub(super) phase: &'a str,
    pub(super) key: &'a str,
}

impl JournalPath<'_> {
    pub(super) fn refusal(self, detail: &str) -> StoreError {
        StoreError::JournalDescriptor {
            phase: self.phase.to_string(),
            key: self.key.to_string(),
            detail: detail.to_string(),
        }
    }

    fn decode(self, what: &str, source: serde_json::Error) -> StoreError {
        StoreError::Decode {
            key: format!("journal object {what} for {}/{}", self.phase, self.key),
            source,
        }
    }

    fn chunk_length(self, index: usize, declared: usize, observed: usize) -> StoreError {
        StoreError::JournalChunkLength {
            phase: self.phase.to_string(),
            key: self.key.to_string(),
            index,
            declared,
            observed,
        }
    }

    fn chunk_digest(self, index: usize, declared: &str, computed: &str) -> StoreError {
        StoreError::JournalChunkDigest {
            phase: self.phase.to_string(),
            key: self.key.to_string(),
            index,
            declared: declared.to_string(),
            computed: computed.to_string(),
        }
    }
}

pub(super) fn alias_object(path: JournalPath<'_>, raw: &[u8]) -> StoreResult<String> {
    let value = record(path, "alias descriptor", raw)?;
    require_type(path, "alias descriptor", &value, OBJECT_ALIAS_TYPE)?;
    require_version(path, "alias descriptor", &value)?;
    let Some(state) = value.get("state").and_then(Value::as_str) else {
        return Err(path.refusal("alias descriptor has no state"));
    };
    if state != OBJECT_STATE_READY {
        return Err(StoreError::JournalNotReady {
            phase: path.phase.to_string(),
            key: path.key.to_string(),
            state: state.to_string(),
        });
    }
    match value
        .get("object")
        .and_then(Value::as_str)
        .filter(|object| is_object_id(object))
    {
        Some(object) => Ok(object.to_string()),
        None => Err(path.refusal("alias descriptor has no 64-hex object id")),
    }
}

pub(super) fn manifest_bounds(
    path: JournalPath<'_>,
    object: &str,
    raw: &[u8],
) -> StoreResult<ObjectBounds> {
    let value = record(path, "manifest", raw)?;
    require_type(path, "manifest", &value, OBJECT_MANIFEST_TYPE)?;
    require_version(path, "manifest", &value)?;
    if value.get("object").and_then(Value::as_str) != Some(object) {
        return Err(path.refusal("manifest object does not match the alias"));
    }
    if value.get("codec").and_then(Value::as_str) != Some(OBJECT_CODEC_JSONL) {
        return Err(path.refusal("manifest codec is not jsonl"));
    }
    let total_bytes = bounded(
        "journal object bytes",
        declared(path, &value, "total_bytes")?,
        OBJECT_BYTES_LIMIT,
    )?;
    let chunks = bounded(
        "journal object chunks",
        declared(path, &value, "chunks")?,
        OBJECT_CHUNKS_LIMIT,
    )?;
    if chunks != expected_chunk_count(total_bytes) {
        return Err(path.refusal("manifest chunk count does not cover its byte length"));
    }
    let records = bounded(
        "journal object records",
        declared(path, &value, "records")?,
        OBJECT_RECORDS_LIMIT,
    )?;
    let index_pages = usize::try_from(declared(path, &value, "index_pages")?)
        .map_err(|_| StoreError::CounterOverflow)?;
    if index_pages != expected_index_pages(records) {
        return Err(path.refusal("manifest index pages do not cover its records"));
    }
    let chunk_digests = digests(path, &value, chunks)?;
    Ok(ObjectBounds {
        total_bytes,
        chunks,
        records,
        index_pages,
        chunk_digests,
    })
}

pub(super) fn index_ranges(
    path: JournalPath<'_>,
    object: &str,
    page: usize,
    first_sequence: usize,
    expected_entries: usize,
    raw: &[u8],
) -> StoreResult<Vec<RecordRange>> {
    let value = record(path, "index page", raw)?;
    require_type(path, "index page", &value, OBJECT_INDEX_TYPE)?;
    require_version(path, "index page", &value)?;
    if value.get("object").and_then(Value::as_str) != Some(object) {
        return Err(path.refusal("index page object does not match the alias"));
    }
    let declared_page = usize::try_from(declared(path, &value, "page")?)
        .map_err(|_| StoreError::CounterOverflow)?;
    if declared_page != page {
        return Err(path.refusal("index page ordinal does not match its key"));
    }
    let Some(entries) = value.get("entries").and_then(Value::as_array) else {
        return Err(path.refusal("index page has no entry array"));
    };
    if entries.len() != expected_entries {
        return Err(path.refusal("index page holds an unexpected number of entries"));
    }
    let mut ranges = Vec::new();
    ranges
        .try_reserve_exact(expected_entries)
        .map_err(|source| StoreError::Refused {
            detail: format!(
                "journal index page of {expected_entries} entries cannot be reserved: {source}"
            ),
        })?;
    for (position, entry) in entries.iter().enumerate() {
        let sequence = usize::try_from(declared(path, entry, "sequence")?)
            .map_err(|_| StoreError::CounterOverflow)?;
        let expected = first_sequence
            .checked_add(position)
            .ok_or(StoreError::CounterOverflow)?;
        if sequence != expected {
            return Err(path.refusal("index entry sequence does not match its position"));
        }
        let offset = usize::try_from(declared(path, entry, "offset")?)
            .map_err(|_| StoreError::CounterOverflow)?;
        let length = usize::try_from(declared(path, entry, "length")?)
            .map_err(|_| StoreError::CounterOverflow)?;
        ranges.push(RecordRange { offset, length });
    }
    Ok(ranges)
}

pub(super) fn chunk_payload(
    path: JournalPath<'_>,
    object: &str,
    index: usize,
    expected_length: usize,
    expected_digest: &str,
    raw: &[u8],
) -> StoreResult<Vec<u8>> {
    let value = record(path, "chunk", raw)?;
    require_type(path, "chunk", &value, OBJECT_CHUNK_TYPE)?;
    require_version(path, "chunk", &value)?;
    if value.get("object").and_then(Value::as_str) != Some(object) {
        return Err(path.refusal("chunk object does not match the alias"));
    }
    let chunk_index = usize::try_from(declared(path, &value, "index")?)
        .map_err(|_| StoreError::CounterOverflow)?;
    if chunk_index != index {
        return Err(path.refusal("chunk ordinal does not match its key"));
    }
    let declared_length = usize::try_from(declared(path, &value, "length")?)
        .map_err(|_| StoreError::CounterOverflow)?;
    if declared_length != expected_length {
        return Err(path.chunk_length(index, expected_length, declared_length));
    }
    let Some(data) = value.get("data").and_then(Value::as_str) else {
        return Err(path.refusal("chunk has no base64 data"));
    };
    let Some(bytes) = decode_object_chunk(data) else {
        return Err(path.refusal("chunk data is not valid base64"));
    };
    if bytes.len() != expected_length {
        return Err(path.chunk_length(index, expected_length, bytes.len()));
    }
    let computed = object_digest(&bytes);
    let Some(record_digest) = value.get("digest").and_then(Value::as_str) else {
        return Err(path.refusal("chunk has no digest"));
    };
    if record_digest != computed {
        return Err(path.chunk_digest(index, record_digest, &computed));
    }
    if computed != expected_digest {
        return Err(path.chunk_digest(index, expected_digest, &computed));
    }
    Ok(bytes)
}

fn record(path: JournalPath<'_>, what: &str, raw: &[u8]) -> StoreResult<Value> {
    serde_json::from_slice(raw).map_err(|source| path.decode(what, source))
}

fn require_type(
    path: JournalPath<'_>,
    what: &str,
    value: &Value,
    expected: &str,
) -> StoreResult<()> {
    if value.get("type").and_then(Value::as_str) == Some(expected) {
        return Ok(());
    }
    Err(path.refusal(&format!("{what} record is not a {expected}")))
}

fn require_version(path: JournalPath<'_>, what: &str, value: &Value) -> StoreResult<()> {
    if value.get("version").and_then(Value::as_u64) == Some(u64::from(JOURNAL_OBJECT_VERSION)) {
        return Ok(());
    }
    Err(path.refusal(&format!(
        "{what} record is not version {JOURNAL_OBJECT_VERSION}"
    )))
}

fn declared(path: JournalPath<'_>, value: &Value, field: &str) -> StoreResult<u64> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| path.refusal(&format!("record has no {field}")))
}

fn bounded(what: &'static str, declared: u64, limit: u64) -> StoreResult<usize> {
    if declared > limit {
        return Err(StoreError::BudgetExceeded {
            what,
            needed: declared,
            available: limit,
        });
    }
    usize::try_from(declared).map_err(|_| StoreError::CounterOverflow)
}

fn digests(path: JournalPath<'_>, value: &Value, chunks: usize) -> StoreResult<Vec<String>> {
    let Some(entries) = value.get("chunk_digests").and_then(Value::as_array) else {
        return Err(path.refusal("manifest has no chunk digest array"));
    };
    if entries.len() != chunks {
        return Err(path.refusal("manifest chunk digests do not cover its chunks"));
    }
    entries
        .iter()
        .map(
            |entry| match entry.as_str().filter(|digest| is_object_id(digest)) {
                Some(digest) => Ok(digest.to_string()),
                None => Err(path.refusal("manifest holds a malformed chunk digest")),
            },
        )
        .collect()
}
