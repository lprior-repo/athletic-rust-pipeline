use fjall::{Keyspace, OwnedWriteBatch, Readable, Snapshot};
use serde::{Deserialize, Serialize};

use super::{
    invalid, PublicIdentityInputRevisionTokenV2, PublicIdentityInputRevisionsV2, KEY, PREFIX,
};
use crate::{StoreError, StoreResult};

const MAX_BYTES: usize = 512;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    version: u32,
    baseline_evidence_generation: u64,
    baseline_derived_generation: u64,
    counters: [u64; 6],
}

pub(crate) fn read(
    snapshot: &Snapshot,
    meta: &Keyspace,
) -> StoreResult<PublicIdentityInputRevisionsV2> {
    let mut rows = snapshot.prefix(meta, PREFIX);
    let Some(first) = rows.next() else {
        return Ok(PublicIdentityInputRevisionsV2::Disabled);
    };
    let key = first.key().map_err(|source| StoreError::Read { source })?;
    if key.as_ref() != KEY.as_bytes() {
        return Err(invalid("unknown capability metadata key"));
    }
    if let Some(second) = rows.next() {
        second.key().map_err(|source| StoreError::Read { source })?;
        return Err(invalid("additional capability metadata key"));
    }
    let length = snapshot
        .size_of(meta, KEY)
        .map_err(|source| StoreError::Read { source })?
        .ok_or_else(|| invalid("capability record disappeared from pinned snapshot"))?;
    let bytes = usize::try_from(length).map_err(|_| StoreError::CounterOverflow)?;
    if bytes > MAX_BYTES {
        return Err(StoreError::RowTooLarge {
            bytes,
            max: MAX_BYTES,
        });
    }
    let raw = snapshot
        .get(meta, KEY)
        .map_err(|source| StoreError::Read { source })?
        .ok_or_else(|| invalid("capability record disappeared from pinned snapshot"))?;
    decode(&raw).map(PublicIdentityInputRevisionsV2::Enabled)
}

fn decode(raw: &[u8]) -> StoreResult<PublicIdentityInputRevisionTokenV2> {
    let record: Record = serde_json::from_slice(raw).map_err(|source| StoreError::Json {
        detail: format!("decoding {KEY}"),
        source,
    })?;
    if record.version != 2 {
        return Err(invalid("unsupported capability record version"));
    }
    Ok(PublicIdentityInputRevisionTokenV2 {
        baseline_evidence_generation: record.baseline_evidence_generation,
        baseline_derived_generation: record.baseline_derived_generation,
        counters: record.counters,
    })
}

pub(super) fn write(
    batch: &mut OwnedWriteBatch,
    meta: &Keyspace,
    token: PublicIdentityInputRevisionTokenV2,
) -> StoreResult<()> {
    batch.insert(meta, KEY, encode(token)?);
    Ok(())
}

pub(crate) fn encode(token: PublicIdentityInputRevisionTokenV2) -> StoreResult<Vec<u8>> {
    let record = Record {
        version: 2,
        baseline_evidence_generation: token.baseline_evidence_generation,
        baseline_derived_generation: token.baseline_derived_generation,
        counters: token.counters,
    };
    let raw = serde_json::to_vec(&record).map_err(|source| StoreError::Json {
        detail: format!("encoding {KEY}"),
        source,
    })?;
    if raw.len() > MAX_BYTES {
        return Err(StoreError::RowTooLarge {
            bytes: raw.len(),
            max: MAX_BYTES,
        });
    }
    Ok(raw)
}

pub(crate) fn decode_attachment(raw: &[u8]) -> StoreResult<PublicIdentityInputRevisionTokenV2> {
    if raw.len() > MAX_BYTES {
        return Err(StoreError::RowTooLarge {
            bytes: raw.len(),
            max: MAX_BYTES,
        });
    }
    let token = decode(raw)?;
    if encode(token)? != raw {
        return Err(invalid("noncanonical attachment input token"));
    }
    Ok(token)
}
