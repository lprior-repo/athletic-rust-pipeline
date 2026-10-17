use super::{has_prefix, validate};
use crate::public_identity::is_qualification_lifecycle;
use crate::{StoreError, StoreResult};
use fjall::{Database, Keyspace, Readable, Snapshot};

pub(crate) fn preflight_keyspaces(db: &Database) -> StoreResult<()> {
    let snapshot = db.snapshot();
    let relevant = [crate::META, crate::JOURNAL, crate::RECEIPTS]
        .into_iter()
        .try_fold(false, |relevant, name| {
            if !db.keyspace_exists(name) {
                return Ok::<bool, StoreError>(relevant);
            }
            let keyspace = db
                .keyspace(name, fjall::KeyspaceCreateOptions::default)
                .map_err(|source| StoreError::Open { source })?;
            let prefix = "public_identity";
            let marked = has_prefix(&snapshot, &keyspace, prefix)?;
            let lifecycle = if name == crate::META {
                raw_lifecycle(&snapshot, &keyspace)?
            } else {
                None
            };
            Ok(relevant || marked || is_qualification_lifecycle(lifecycle))
        })?;
    if relevant {
        crate::format::existing::require_keyspaces(db)?;
    }
    Ok(())
}

fn raw_lifecycle(snapshot: &Snapshot, meta: &Keyspace) -> StoreResult<Option<u32>> {
    let key = crate::format::LIFECYCLE_REVISION_KEY;
    let size = snapshot
        .size_of(meta, key)
        .map_err(|source| StoreError::Read { source })?;
    if !size.is_some_and(|size| size <= 32) {
        return Ok(None);
    }
    let raw = snapshot
        .get(meta, key)
        .map_err(|source| StoreError::Read { source })?;
    Ok(raw
        .as_ref()
        .and_then(|raw| std::str::from_utf8(raw).ok())
        .and_then(|value| value.trim().parse().ok()))
}

pub(crate) fn validate_preservation(
    root: &std::path::Path,
    db: &Database,
    entities: &Keyspace,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
) -> StoreResult<()> {
    let snapshot = db.snapshot();
    crate::roster_source_issuance::validate_format(
        &snapshot,
        meta,
        journal,
        receipts,
        &crate::format::read_format(meta)?,
        &mut crate::RosterStartupValidationBudgetV1::new(),
    )?;
    if !is_qualification_lifecycle(raw_lifecycle(&snapshot, meta)?)
        && !has_prefix(&snapshot, meta, "public_identity")?
        && !has_prefix(&snapshot, journal, "public_identity")?
        && !has_prefix(&snapshot, receipts, "public_identity")?
    {
        return Ok(());
    }
    let format = crate::format::require_current(root, crate::format::read_format(meta)?)?;
    validate(&snapshot, entities, meta, journal, receipts, &format)?;
    Ok(())
}
