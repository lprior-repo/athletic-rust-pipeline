use fjall::{Database, Keyspace, KeyspaceCreateOptions, Readable, Snapshot};

use super::{codec, invalid, PublicIdentityInputRevisionsV2};
use crate::{StoreError, StoreFormat, StoreResult};

pub(crate) fn preflight_keyspaces(root: &std::path::Path, db: &Database) -> StoreResult<()> {
    if !db.keyspace_exists(crate::META) {
        return Ok(());
    }
    let meta = db
        .keyspace(crate::META, KeyspaceCreateOptions::default)
        .map_err(|source| StoreError::Open { source })?;
    let snapshot = db.snapshot();
    let Some(first) = snapshot.prefix(&meta, super::PREFIX).next() else {
        return Ok(());
    };
    first.key().map_err(|source| StoreError::Read { source })?;
    crate::format::existing::require_keyspaces(db)?;
    let format = crate::format::require_current(root, crate::format::read_format(&meta)?)?;
    validate_format(&snapshot, &meta, &format)
}

pub(crate) fn validate_format(
    snapshot: &Snapshot,
    meta: &Keyspace,
    format: &StoreFormat,
) -> StoreResult<()> {
    let PublicIdentityInputRevisionsV2::Enabled(token) = codec::read(snapshot, meta)? else {
        return Ok(());
    };
    if format.schema_version != Some(crate::STORE_SCHEMA_VERSION)
        || format.key_format != Some(crate::KEY_FORMAT_VERSION)
        || format.migration_target.is_some()
        || format.created_by.is_none()
        || format.created_at.is_none()
    {
        return Err(invalid(
            "capability requires the current unmigrating store format",
        ));
    }
    validate_baseline(snapshot, meta, token)
}

pub(super) fn validate_baseline(
    snapshot: &Snapshot,
    meta: &Keyspace,
    token: super::PublicIdentityInputRevisionTokenV2,
) -> StoreResult<()> {
    let evidence = generation(snapshot, meta, crate::generation::ENTITIES)?;
    let derived = generation(snapshot, meta, crate::generation::DERIVED_CURRENT)?;
    if token.baseline_evidence_generation > evidence || token.baseline_derived_generation > derived
    {
        return Err(invalid(
            "enrollment baseline is newer than the store generations",
        ));
    }
    Ok(())
}

fn generation(snapshot: &Snapshot, meta: &Keyspace, key: &str) -> StoreResult<u64> {
    let raw = snapshot
        .get(meta, key)
        .map_err(|source| StoreError::Read { source })?
        .ok_or_else(|| invalid("missing generation metadata"))?;
    let text = std::str::from_utf8(&raw).map_err(|_| invalid("invalid generation metadata"))?;
    crate::meta::parse_u64(key, text)
}
