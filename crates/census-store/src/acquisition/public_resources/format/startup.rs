use super::*;
use crate::acquisition::public_resources::read;
use crate::acquisition::{AcquisitionReadBudget, CaptureError};
use crate::{StoreError, StoreFormat};
use fjall::{Keyspace, Readable, Snapshot};

pub(crate) fn validate_format(
    snapshot: &Snapshot,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    format: &StoreFormat,
) -> StoreResult<()> {
    if !has_state(snapshot, meta, journal, receipts) {
        return Ok(());
    }
    let mut budget = AcquisitionReadBudget::new(4096, 32 * 1024 * 1024, 0)?;
    scan(snapshot, meta, &mut budget, |_| {
        Err(refused("public resource state in metadata"))
    })?;
    let declaration = read::declaration(snapshot, meta, journal, receipts, format, &mut budget)?
        .ok_or(CaptureError::CaptureCapabilityNamespaceUnsupported)?;
    scan(snapshot, journal, &mut budget, |(key, budget)| {
        if key.starts_with(b"public_resource_capture") {
            return validate_declaration_key(key);
        }
        read::entry(snapshot, journal, receipts, key, &declaration, budget).map(|_| ())
    })?;
    scan(snapshot, receipts, &mut budget, |(key, budget)| {
        validate_receipt(snapshot, journal, receipts, key, &declaration, budget)
    })?;
    crate::acquisition::public_resources::child::startup::validate_all(
        snapshot,
        meta,
        journal,
        receipts,
        &declaration,
    )?;
    crate::acquisition::public_resources::intake::startup::validate_all(
        snapshot,
        meta,
        journal,
        receipts,
        &declaration,
    )
}

pub(crate) fn has_state(
    snapshot: &Snapshot,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
) -> bool {
    [meta, journal, receipts].iter().any(|table| {
        [
            b"public_resource_capture".as_slice(),
            b"public_resource_frontier".as_slice(),
            b"public_resource_child".as_slice(),
            b"public_resource_school".as_slice(),
        ]
        .iter()
        .any(|prefix| snapshot.prefix(table, prefix).next().is_some())
    })
}

fn scan(
    snapshot: &Snapshot,
    table: &Keyspace,
    budget: &mut AcquisitionReadBudget,
    mut validate: impl FnMut((&[u8], &mut AcquisitionReadBudget)) -> StoreResult<()>,
) -> StoreResult<()> {
    let prefixes = [
        b"public_resource_capture".as_slice(),
        b"public_resource_frontier".as_slice(),
    ];
    for prefix in prefixes {
        for (position, guard) in snapshot.prefix(table, prefix).take(104).enumerate() {
            if position >= 103 {
                return Err(refused("public resource root inventory limit"));
            }
            let key = guard.key().map_err(|source| StoreError::Read { source })?;
            if key.len() > crate::MAX_JOURNAL_KEY_BYTES {
                return Err(CaptureError::RecordTooLarge.into());
            }
            budget.encoded(key.len())?;
            validate((&key, budget))?;
        }
    }
    Ok(())
}

fn validate_receipt(
    snapshot: &Snapshot,
    journal: &Keyspace,
    receipts: &Keyspace,
    key: &[u8],
    declaration: &PublicResourceDeclarationV1,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<()> {
    if key == declaration.receipt()?.operation.as_bytes() {
        let receipt = read::record(snapshot, receipts, key, budget)?
            .ok_or_else(|| refused("public resource declaration receipt absent"))?;
        return declaration.validate(&receipt);
    }
    let id = key
        .strip_prefix(FRONTIER_OPERATION_PREFIX.as_bytes())
        .and_then(|id| std::str::from_utf8(id).ok())
        .ok_or_else(|| refused("public resource receipt namespace"))?;
    let id = census_domain::model::AcquisitionDigest::parse(id).map_err(CaptureError::Domain)?;
    read::entry(
        snapshot,
        journal,
        receipts,
        format!("{ENTRY_PREFIX}{id}").as_bytes(),
        declaration,
        budget,
    )
    .map(|_| ())
}
