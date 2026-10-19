use super::*;
use census_domain::model::StoreIdentity;
use fjall::Readable;

const VALUE_BYTES: usize = 2 * MAX_RESULT_CALENDAR_RECORD_BYTES;

pub(in super::super) fn value(
    snapshot: &Snapshot,
    table: &Keyspace,
    key: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    bounded(snapshot, table, key, VALUE_BYTES, grant)
}

pub(in super::super) fn entry_value(
    snapshot: &Snapshot,
    table: &Keyspace,
    key: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    bounded(
        snapshot,
        table,
        key,
        census_domain::public_identity_history::ReplayLimits::SMALL.total_text_bytes,
        grant,
    )
}

fn bounded(
    snapshot: &Snapshot,
    table: &Keyspace,
    key: &[u8],
    maximum: usize,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    grant.charge_steps(2).map_err(grant_error)?;
    let length = snapshot
        .size_of(table, key)
        .map_err(|source| StoreError::Read { source })?
        .ok_or(Error::Corrupt("missing protected value"))?;
    let length = usize::try_from(length).map_err(|_| Error::BudgetExceeded)?;
    if length > maximum {
        return Err(Error::BudgetExceeded.into());
    }
    grant
        .charge_bytes(
            length
                .checked_mul(2)
                .and_then(|size| size.checked_add(key.len()))
                .ok_or(Error::BudgetExceeded)?,
        )
        .map_err(grant_error)?;
    let raw = snapshot
        .get(table, key)
        .map_err(|source| StoreError::Read { source })?
        .ok_or(Error::Corrupt("missing protected value"))?;
    if raw.len() != length {
        return Err(Error::Corrupt("protected value size changed").into());
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length)
        .map_err(|_| Error::BudgetExceeded)?;
    bytes.extend_from_slice(&raw);
    Ok(bytes)
}

pub(in super::super) fn namespace(
    snapshot: &Snapshot,
    table: &Keyspace,
    prefix: &str,
    expected: &[&str],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let mut rows = snapshot.prefix(table, prefix);
    for expected in expected {
        grant.charge_steps(1).map_err(grant_error)?;
        let row = rows
            .next()
            .ok_or(Error::Corrupt("missing protected namespace key"))?;
        let key = row.key().map_err(|source| StoreError::Read { source })?;
        grant.charge_bytes(key.len()).map_err(grant_error)?;
        if expected.as_bytes() != key.as_ref() {
            return Err(Error::Corrupt("unexpected protected namespace key").into());
        }
    }
    grant.charge_steps(1).map_err(grant_error)?;
    if let Some(row) = rows.next() {
        row.key().map_err(|source| StoreError::Read { source })?;
        return Err(Error::Corrupt("extra protected namespace key").into());
    }
    Ok(())
}

pub(in super::super) fn identity(
    snapshot: &Snapshot,
    journal: &Keyspace,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<StoreIdentity> {
    let key = crate::Store::journal_key("export-lineage-v1", "store-identity");
    value(snapshot, journal, &key, grant)?;
    grant.charge_steps(64).map_err(grant_error)?;
    match crate::lineage::read(snapshot, journal)? {
        crate::lineage::RetainedIdentity::Present(identity) => Ok(identity),
        crate::lineage::RetainedIdentity::Absent => {
            Err(Error::Corrupt("origin lineage absent").into())
        }
    }
}

pub(in super::super) fn snapshot_format(
    snapshot: &Snapshot,
    meta: &Keyspace,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<StoreFormat> {
    let mut lifecycle_revision = None;
    for (key, expected) in [
        ("schema:store_version", Some(b"2".as_slice())),
        ("schema:key_format", Some(b"1".as_slice())),
        ("schema:lifecycle_revision", None),
        ("schema:migrating_to", None),
    ] {
        grant.charge_steps(2).map_err(grant_error)?;
        let size = snapshot
            .size_of(meta, key)
            .map_err(|source| StoreError::Read { source })?;
        if size.is_some_and(|size| size > 32) {
            return Err(Error::BudgetExceeded.into());
        }
        grant.charge_bytes(32).map_err(grant_error)?;
        let raw = snapshot
            .get(meta, key)
            .map_err(|source| StoreError::Read { source })?;
        let bytes = raw.as_ref().map(|raw| raw.as_ref());
        if key == "schema:lifecycle_revision" {
            lifecycle_revision = match bytes {
                Some(b"7") => Some(7),
                Some(b"8") => Some(8),
                Some(b"9") => Some(9),
                Some(b"10") => Some(10),
                _ => return Err(Error::EnrollmentRequired.into()),
            };
        } else if bytes != expected {
            return Err(Error::EnrollmentRequired.into());
        }
    }
    Ok(StoreFormat {
        schema_version: Some(2),
        key_format: Some(1),
        lifecycle_revision,
        created_by: None,
        created_at: None,
        migration_target: None,
    })
}
