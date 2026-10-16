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

mod stream;

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
    encoded_rows(&envelope, &key, grant)
}

fn encoded_rows(
    envelope: &impl Serialize,
    key: &str,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<(Vec<u8>, Vec<u8>)>> {
    let measured = encoding::measure(envelope, grant)?;
    if measured > MAX_JOURNAL_VALUE_BYTES {
        return stream::rows(envelope, key, physical_key(key, grant)?, measured, grant);
    }
    let (_, raw) = encoding::object_measured(envelope, measured, grant)?.into_parts();
    let physical = physical_key(key, grant)?;
    let mut rows = Vec::new();
    super::material::allocate(&mut rows)?;
    rows.push((physical, raw));
    Ok(rows)
}

fn physical_key(key: &str, grant: &mut AttachmentReplayGrantV1) -> StoreResult<Vec<u8>> {
    let key_bytes = PHASE
        .len()
        .checked_add(key.len())
        .and_then(|n| n.checked_add(1))
        .ok_or(StoreError::CounterOverflow)?;
    grant.charge_bytes(key_bytes).map_err(Error::Domain)?;
    Ok(Store::journal_key(PHASE, key))
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
