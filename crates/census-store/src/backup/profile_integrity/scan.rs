use super::key::{self, Entry};
use super::{classify, CorruptionClass, Report, Scope, PREFIX};
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
    for guard in snapshot.prefix(meta, PREFIX).take(bound) {
        let raw = guard.key().map_err(|source| StoreError::Read { source })?;
        let size = snapshot
            .size_of(meta, &raw)
            .map_err(|source| StoreError::Read { source })?
            .ok_or_else(|| StoreError::Invariant {
                detail: "profile inventory key vanished from its captured snapshot".to_string(),
            })?;
        if !report.admit(raw.len(), size)? {
            return Ok(());
        }
        inspect(raw.as_ref(), size, report)?;
        if report.scope == Scope::Incomplete {
            return Ok(());
        }
    }
    Ok(())
}

fn inspect(raw: &[u8], size: u32, report: &mut Report) -> StoreResult<()> {
    if !key::supported_namespace(raw) {
        report.incomplete(format!("{} uses an unsupported profile namespace; verification is incomplete, not a corruption diagnosis", super::label(raw)));
        return Ok(());
    }
    if raw.len() > super::MAX_LABEL_BYTES {
        report.note(
            CorruptionClass::Oversized,
            raw,
            "profile key exceeds the supported 128-byte framing envelope".to_string(),
        );
        return Ok(());
    }
    if u64::from(size)
        > u64::try_from(MAX_PROFILE_RECORD_BYTES).map_err(|_| StoreError::CounterOverflow)?
    {
        report.note(CorruptionClass::Oversized, raw, format!("physical profile record holds {size} bytes, exceeding {MAX_PROFILE_RECORD_BYTES}; value was not retrieved"));
        return Ok(());
    }
    match key::parse(raw)? {
        Entry::Unsupported => report.incomplete(format!("{} uses an unsupported profile namespace; verification is incomplete, not a corruption diagnosis", super::label(raw))),
        Entry::Malformed => report.note(CorruptionClass::Undecodable, raw, "profile key is not canonical profile framing".to_string()),
        Entry::Profile(_) => {}
    }
    Ok(())
}

pub(super) fn decode(snapshot: &Snapshot, meta: &Keyspace, report: &mut Report) -> StoreResult<()> {
    let bound = usize::try_from(super::MAX_RECORDS).map_err(|_| StoreError::CounterOverflow)?;
    for guard in snapshot.prefix(meta, PREFIX).take(bound) {
        let raw = guard.key().map_err(|source| StoreError::Read { source })?;
        if raw.len() <= super::MAX_LABEL_BYTES {
            decode_one(snapshot, meta, raw.as_ref(), report)?;
        }
        if report.scope == Scope::Incomplete {
            return Ok(());
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
            detail: "profile record vanished from its captured snapshot".to_string(),
        })?;
    if u64::from(size)
        > u64::try_from(MAX_PROFILE_RECORD_BYTES).map_err(|_| StoreError::CounterOverflow)?
    {
        return Ok(());
    }
    let Entry::Profile(id) = key::parse(raw)? else {
        return Ok(());
    };
    if let Err(error) = crate::profiles::read::retained(snapshot, meta, &id).and_then(|profile| {
        profile
            .ok_or_else(|| crate::ProfileRegistryError::MissingProfile { id }.into())
            .map(|_| ())
    }) {
        classify::record(report, raw, error)?;
    }
    Ok(())
}
