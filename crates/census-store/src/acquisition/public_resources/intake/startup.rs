use super::{
    admission, codec, refused, Mint, PublicSchoolIntakePacketV1, MINT_PREFIX, OPERATION_PREFIX,
    PHASE,
};
use crate::acquisition::public_resources::PublicResourceDeclarationV1;
use crate::acquisition::AcquisitionReadBudget;
use crate::{Receipt, Store, StoreError, StoreResult};
use fjall::{Keyspace, Readable, Snapshot};
use std::collections::BTreeSet;

pub(crate) fn validate_all(
    snapshot: &Snapshot,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    declaration: &PublicResourceDeclarationV1,
) -> StoreResult<()> {
    let maximum = declaration
        .binding
        .owners
        .len()
        .checked_mul(super::super::child::MAX_TEAMS)
        .ok_or(StoreError::CounterOverflow)?;
    let mut budget = AcquisitionReadBudget::inventory();
    let mut roots = roots(snapshot, journal, maximum, &mut budget)?;
    for key in keys(
        snapshot,
        meta,
        b"public_resource_school",
        maximum,
        &mut budget,
    )? {
        let key =
            std::str::from_utf8(&key).map_err(|_| refused("public school intake mint key"))?;
        let id = key
            .strip_prefix(MINT_PREFIX)
            .ok_or_else(|| refused("public school intake mint namespace"))?;
        let packet = read_packet(snapshot, journal, receipts, id)?;
        let mint = Mint::from_packet(&packet)?;
        if mint.key() != key || packet.wire.declaration != *declaration || !roots.remove(id) {
            return Err(refused("public school intake root mint linkage"));
        }
        let mint_bytes = admission::raw(snapshot, meta, key.as_bytes(), &mut budget)?;
        let operation = format!("{OPERATION_PREFIX}{id}");
        let receipt_bytes = admission::raw(snapshot, receipts, operation.as_bytes(), &mut budget)?;
        super::PublicSchoolIntakeAdmissionV1::from_retained(&packet, &mint_bytes, &receipt_bytes)?;
    }
    if !roots.is_empty() {
        return Err(refused("public school intake root lacks mint"));
    }
    validate_receipts(snapshot, meta, receipts, maximum, &mut budget)
}

fn read_packet(
    snapshot: &Snapshot,
    journal: &Keyspace,
    receipts: &Keyspace,
    id: &str,
) -> StoreResult<PublicSchoolIntakePacketV1> {
    let source = crate::read::PhysicalRetainedJournalSource::new(snapshot, journal, receipts);
    let root = crate::evidence_subject_attachment::read_roster_retained_capture_root_v1(
        &source,
        crate::RetainedCaptureRootSchemaV1::ProviderSchoolObservedPacketV1,
        id,
        &mut census_domain::evidence_subject_attachment::AttachmentReplayGrantV1::new(),
    )?
    .ok_or_else(|| refused("public school intake root absent"))?;
    let packet = PublicSchoolIntakePacketV1::decode(root.payload().get().as_bytes())?;
    if packet.reference().as_str() != id {
        return Err(refused("public school intake root key differs"));
    }
    Ok(packet)
}

fn roots(
    snapshot: &Snapshot,
    journal: &Keyspace,
    maximum: usize,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<BTreeSet<String>> {
    let prefix = Store::journal_key(PHASE, "");
    let rows =
        usize::try_from(crate::evidence_subject_attachment::MAX_RETAINED_CAPTURE_PHYSICAL_ROWS)
            .map_err(|_| StoreError::CounterOverflow)?;
    let physical = maximum
        .checked_mul(rows)
        .ok_or(StoreError::CounterOverflow)?;
    let mut roots = BTreeSet::new();
    for key in keys(
        snapshot,
        journal,
        b"public_resource_school",
        physical,
        budget,
    )? {
        let logical = logical_root(&key, rows)?;
        let suffix = logical
            .strip_prefix(prefix.as_slice())
            .and_then(|value| std::str::from_utf8(value).ok())
            .ok_or_else(|| refused("public school intake physical namespace"))?;
        let id = census_domain::model::AcquisitionDigest::parse(suffix)
            .map_err(crate::CaptureError::Domain)?;
        roots.insert(id.as_str().to_string());
        if roots.len() > maximum {
            return Err(refused("public school intake inventory limit"));
        }
    }
    Ok(roots)
}

fn logical_root(key: &[u8], rows: usize) -> StoreResult<&[u8]> {
    let Some((root, index)) = crate::journal::parse_chunk_key(key) else {
        return Ok(key);
    };
    if index >= rows.checked_sub(1).ok_or(StoreError::CounterOverflow)?
        || crate::journal::chunk_key(root, index)? != key
    {
        return Err(refused("public school intake chunk namespace"));
    }
    Ok(root)
}

fn keys(
    snapshot: &Snapshot,
    table: &Keyspace,
    prefix: &[u8],
    maximum: usize,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<Vec<Vec<u8>>> {
    let mut keys = Vec::new();
    for (position, guard) in snapshot.prefix(table, prefix).take(maximum + 1).enumerate() {
        if position >= maximum {
            return Err(refused("public school intake inventory limit"));
        }
        let key = guard.key().map_err(|source| StoreError::Read { source })?;
        if key.len() > crate::MAX_JOURNAL_KEY_BYTES {
            return Err(crate::CaptureError::RecordTooLarge.into());
        }
        budget.encoded(key.len())?;
        let mut held = Vec::new();
        held.try_reserve_exact(key.len())
            .map_err(|_| refused("public school intake key allocation"))?;
        held.extend_from_slice(&key);
        keys.try_reserve(1)
            .map_err(|_| refused("public school intake key inventory allocation"))?;
        keys.push(held);
    }
    Ok(keys)
}

fn validate_receipts(
    snapshot: &Snapshot,
    meta: &Keyspace,
    receipts: &Keyspace,
    maximum: usize,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<()> {
    for key in keys(
        snapshot,
        receipts,
        b"public_resource_school",
        maximum,
        budget,
    )? {
        let operation =
            std::str::from_utf8(&key).map_err(|_| refused("public school intake receipt key"))?;
        let id = operation
            .strip_prefix(OPERATION_PREFIX)
            .ok_or_else(|| refused("public school intake receipt namespace"))?;
        let mint: Mint = codec::decode(&admission::raw(
            snapshot,
            meta,
            format!("{MINT_PREFIX}{id}").as_bytes(),
            budget,
        )?)?;
        let receipt: Receipt = codec::decode(&admission::raw(snapshot, receipts, &key, budget)?)?;
        if mint.packet.as_str() != id || receipt.operation != operation {
            return Err(refused("public school intake receipt lacks matching mint"));
        }
    }
    Ok(())
}
