use super::{
    encoding, keys, ledger::wire, material::Material, EvidenceSubjectAttachmentStoreError as Error,
    PHASE,
};
use crate::{Store, StoreError, StoreResult, MAX_JOURNAL_VALUE_BYTES};
use census_domain::evidence_subject_attachment::{
    AttachmentReplayGrantV1, EvidenceSubjectAttachmentError, ReviewedReferenceRecordV1,
};
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Serialize)]
struct Envelope<'a> {
    at: String,
    key: &'a str,
    payload: wire::Encoded<'a>,
}

pub(super) fn rows(
    ordinal: u64,
    record: &ReviewedReferenceRecordV1,
    introduced: &[Material],
    inputs: &wire::EntryInputs,
    committed_at: DateTime<Utc>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<(Vec<u8>, Vec<u8>)>> {
    grant.charge_steps(1).map_err(Error::Domain)?;
    let key = keys::record(ordinal)?;
    let envelope = Envelope {
        at: committed_at.to_rfc3339(),
        key: &key,
        payload: wire::Encoded {
            schema: wire::SCHEMA,
            record,
            introduced,
            inputs,
        },
    };
    let (_, raw) = encoding::object(&envelope, grant)?.into_parts();
    let key_bytes = PHASE
        .len()
        .checked_add(key.len())
        .and_then(|n| n.checked_add(1))
        .ok_or(StoreError::CounterOverflow)?;
    grant.charge_bytes(key_bytes).map_err(Error::Domain)?;
    let physical = Store::journal_key(PHASE, &key);
    if raw.len() <= MAX_JOURNAL_VALUE_BYTES {
        let mut rows = Vec::new();
        super::material::allocate(&mut rows)?;
        rows.push((physical, raw));
        return Ok(rows);
    }
    chunks(&key, physical, &raw, grant)
}

fn chunks(
    key: &str,
    physical: Vec<u8>,
    raw: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<(Vec<u8>, Vec<u8>)>> {
    let total = raw.len().div_ceil(crate::journal::JOURNAL_CHUNK_RAW);
    grant.charge_bytes(512).map_err(Error::Domain)?;
    grant.charge_steps(2).map_err(Error::Domain)?;
    let manifest = crate::journal::manifest_value(key, total, raw.len())?;
    if manifest.len() > 512 {
        return Err(Error::Corrupt("attachment manifest frame bound").into());
    }
    let mut rows = Vec::new();
    super::material::allocate(&mut rows)?;
    grant.charge_bytes(physical.len()).map_err(Error::Domain)?;
    rows.push((physical.clone(), manifest));
    for (index, chunk) in raw.chunks(crate::journal::JOURNAL_CHUNK_RAW).enumerate() {
        let allowance = chunk
            .len()
            .checked_add(2)
            .and_then(|n| n.checked_div(3))
            .and_then(|n| n.checked_mul(4))
            .and_then(|n| n.checked_add(512))
            .ok_or(StoreError::CounterOverflow)?;
        grant.charge_bytes(allowance).map_err(Error::Domain)?;
        grant.charge_steps(5).map_err(Error::Domain)?;
        let value = crate::journal::chunk_value(key, index, total, chunk)?;
        if value.len() > allowance || value.len() > MAX_JOURNAL_VALUE_BYTES {
            return Err(Error::Corrupt("attachment chunk frame bound").into());
        }
        grant
            .charge_bytes(
                physical
                    .len()
                    .checked_add(5)
                    .ok_or(StoreError::CounterOverflow)?,
            )
            .map_err(Error::Domain)?;
        let key = crate::journal::chunk_key(&physical, index)?;
        super::material::allocate(&mut rows)?;
        rows.push((key, value));
    }
    Ok(rows)
}

pub(super) fn require_complete_fit(
    ledger: &super::ledger::Ledger<'_>,
    rows: &[(Vec<u8>, Vec<u8>)],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let mut bytes = usize::try_from(ledger.physical.fingerprint().encoded_bytes())
        .map_err(|_| StoreError::CounterOverflow)?;
    let count = usize::try_from(ledger.physical.fingerprint().physical_rows())
        .map_err(|_| StoreError::CounterOverflow)?
        .checked_add(rows.len())
        .ok_or(StoreError::CounterOverflow)?;
    for (key, value) in rows {
        grant.charge_steps(1).map_err(Error::Domain)?;
        bytes = bytes
            .checked_add(key.len())
            .and_then(|n| n.checked_add(value.len()))
            .ok_or(StoreError::CounterOverflow)?;
    }
    for (resource, needed, available) in [
        ("attachment physical rows", count, 8192),
        ("attachment physical bytes", bytes, 4 * 1024 * 1024),
    ] {
        if needed > available {
            return Err(Error::Domain(EvidenceSubjectAttachmentError::Capacity {
                resource,
                needed,
                available,
            })
            .into());
        }
    }
    Ok(())
}
