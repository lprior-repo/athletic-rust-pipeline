use serde::Serialize;

use super::{bounded, IndexEntry, ObjectPlan};
use crate::journal::{
    encode_chunk, JOURNAL_OBJECT_VERSION, OBJECT_CHUNK_TYPE, OBJECT_CODEC_JSONL, OBJECT_INDEX_TYPE,
    OBJECT_MANIFEST_TYPE,
};
use crate::{StoreError, StoreResult, MAX_JOURNAL_VALUE_BYTES};

#[derive(Serialize)]
struct Chunk<'a> {
    r#type: &'static str,
    version: u8,
    object: &'a str,
    index: usize,
    length: usize,
    digest: &'a str,
    data: String,
}

#[derive(Serialize)]
struct Page<'a> {
    r#type: &'static str,
    version: u8,
    object: &'a str,
    page: usize,
    entries: &'a [IndexEntry],
}

#[derive(Serialize)]
struct Manifest<'a> {
    r#type: &'static str,
    version: u8,
    object: &'a str,
    codec: &'static str,
    total_bytes: usize,
    chunks: usize,
    records: usize,
    index_pages: usize,
    chunk_digests: &'a [String],
}

pub(super) fn chunk(
    object: &str,
    index: usize,
    bytes: &[u8],
    digest: &str,
) -> StoreResult<Vec<u8>> {
    encode(&Chunk {
        r#type: OBJECT_CHUNK_TYPE,
        version: JOURNAL_OBJECT_VERSION,
        object,
        index,
        length: bytes.len(),
        digest,
        data: encode_chunk(bytes),
    })
}

pub(super) fn page(object: &str, page: usize, entries: &[IndexEntry]) -> StoreResult<Vec<u8>> {
    encode(&Page {
        r#type: OBJECT_INDEX_TYPE,
        version: JOURNAL_OBJECT_VERSION,
        object,
        page,
        entries,
    })
}

pub(super) fn manifest(plan: &ObjectPlan) -> StoreResult<Vec<u8>> {
    encode(&Manifest {
        r#type: OBJECT_MANIFEST_TYPE,
        version: JOURNAL_OBJECT_VERSION,
        object: &plan.staged.object,
        codec: OBJECT_CODEC_JSONL,
        total_bytes: plan.staged.total_bytes,
        chunks: plan.staged.chunks,
        records: plan.staged.records,
        index_pages: plan.page_digests.len(),
        chunk_digests: &plan.chunk_digests,
    })
}

fn encode<T: Serialize>(record: &T) -> StoreResult<Vec<u8>> {
    let bytes = serde_json::to_vec(record).map_err(|source| StoreError::Json {
        detail: "serializing bounded context physical row".to_string(),
        source,
    })?;
    bounded(
        "context physical row bytes",
        bytes.len(),
        MAX_JOURNAL_VALUE_BYTES,
    )?;
    Ok(bytes)
}
