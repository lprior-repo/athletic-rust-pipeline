use super::key::{self, Entry};
use super::{classify, CorruptionClass, Report, Scope, PREFIXES};
use crate::{StoreError, StoreResult, MAX_PROFILE_RECORD_BYTES};
use fjall::{Keyspace, Readable, Snapshot};

pub(super) fn preflight(
    snapshot: &Snapshot,
    meta: &Keyspace,
    report: &mut Report,
) -> StoreResult<()> {
    let bound = super::MAX_RECORDS
        .checked_add(1)
        .ok_or(StoreError::CounterOverflow)?;
    let bound = usize::try_from(bound).map_err(|_| StoreError::CounterOverflow)?;
    for prefix in PREFIXES {
        for guard in snapshot.prefix(meta, prefix).take(bound) {
            let raw = guard.key().map_err(|source| StoreError::Read { source })?;
            let size = snapshot
                .size_of(meta, &raw)
                .map_err(|source| StoreError::Read { source })?
                .ok_or_else(|| StoreError::Invariant {
                    detail: "registry inventory key vanished from its captured snapshot"
                        .to_string(),
                })?;
            if !report.admit(raw.len(), size)? {
                return Ok(());
            }
            inspect(snapshot, meta, raw.as_ref(), size, report)?;
            if report.scope == Scope::Incomplete {
                return Ok(());
            }
        }
    }
    Ok(())
}

fn inspect(
    snapshot: &Snapshot,
    meta: &Keyspace,
    raw: &[u8],
    size: u32,
    report: &mut Report,
) -> StoreResult<()> {
    if !key::supported_namespace(raw) {
        report.incomplete(format!("{} uses an unsupported registry namespace; verification is incomplete, not a corruption diagnosis", super::label(raw)));
        return Ok(());
    }
    if raw.len() > super::MAX_LABEL_BYTES {
        report.note(
            CorruptionClass::Oversized,
            raw,
            "registry key exceeds the supported 128-byte framing envelope".to_string(),
        );
        return Ok(());
    }
    if u64::from(size)
        > u64::try_from(MAX_PROFILE_RECORD_BYTES).map_err(|_| StoreError::CounterOverflow)?
    {
        report.note(CorruptionClass::Oversized, raw, format!("physical registry record holds {size} bytes, exceeding {MAX_PROFILE_RECORD_BYTES}; value was not retrieved"));
        return Ok(());
    }
    match key::parse(raw)? {
        Entry::Chunk { descriptor } => {
            if snapshot.size_of(meta, descriptor).map_err(|source| StoreError::Read { source })?.is_none() {
                report.note(CorruptionClass::Truncated, raw, "retained regional payload chunk has no exact descriptor (orphan payload)".to_string());
            }
        }
        Entry::Unsupported => report.incomplete(format!("{} uses an unsupported registry namespace; verification is incomplete, not a corruption diagnosis", super::label(raw))),
        Entry::Malformed => report.note(CorruptionClass::Undecodable, raw, "registry key is not canonical profile or regional descriptor/chunk framing".to_string()),
        Entry::Profile(_) | Entry::Descriptor(_) => {}
    }
    Ok(())
}

pub(super) fn decode(snapshot: &Snapshot, meta: &Keyspace, report: &mut Report) -> StoreResult<()> {
    let bound = usize::try_from(super::MAX_RECORDS).map_err(|_| StoreError::CounterOverflow)?;
    for prefix in PREFIXES {
        for guard in snapshot.prefix(meta, prefix).take(bound) {
            let raw = guard.key().map_err(|source| StoreError::Read { source })?;
            if raw.len() <= super::MAX_LABEL_BYTES {
                decode_one(snapshot, meta, raw.as_ref(), report)?;
            }
            if report.scope == Scope::Incomplete {
                return Ok(());
            }
        }
    }
    Ok(())
}

fn decode_one(
    snapshot: &Snapshot,
    meta: &Keyspace,
    raw: &[u8],
    report: &mut Report,
) -> StoreResult<()> {
    let size = snapshot
        .size_of(meta, raw)
        .map_err(|source| StoreError::Read { source })?
        .ok_or_else(|| StoreError::Invariant {
            detail: "registry record vanished from its captured snapshot".to_string(),
        })?;
    if u64::from(size)
        > u64::try_from(MAX_PROFILE_RECORD_BYTES).map_err(|_| StoreError::CounterOverflow)?
    {
        return Ok(());
    }
    let result = match key::parse(raw)? {
        Entry::Profile(id) => {
            let value = snapshot
                .get(meta, raw)
                .map_err(|source| StoreError::Read { source })?
                .ok_or_else(|| StoreError::Invariant {
                    detail: "profile vanished from its captured snapshot".to_string(),
                })?;
            crate::profiles::read::decode(&id, value.as_ref()).map(|_| ())
        }
        Entry::Descriptor(reference) => {
            crate::region_contracts::read::read(snapshot, meta, &reference).map(|_| ())
        }
        Entry::Chunk { .. } | Entry::Unsupported | Entry::Malformed => return Ok(()),
    };
    if let Err(error) = result {
        classify::record(report, raw, error)?;
    }
    Ok(())
}
