use super::{walk, wire, FrozenPublicIdentityBundleV2};
use crate::public_identity::{
    codec,
    paged_generation::{graph_physical_rows, ObjectRefV2},
    retained::{Live, Read},
    PublicIdentityError, DECLARATION, HEAD, HISTORY, MAX_PUBLIC_IDENTITY_HISTORY_BYTES,
};
use crate::{AcceptedPublicIdentitySnapshot, StoreError, StoreResult};
use census_domain::public_identity_qualification::PublicIdentityQualificationVersion;
use fjall::{Keyspace, Snapshot};
use std::collections::BTreeMap;

pub(crate) fn capture(
    snapshot: &Snapshot,
    entities: &Keyspace,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    accepted: &AcceptedPublicIdentitySnapshot,
) -> StoreResult<FrozenPublicIdentityBundleV2> {
    require_snapshot(snapshot, accepted)?;
    let reader = Live::new(snapshot, entities, meta, journal, receipts)?;
    let mut bundle = empty_bundle(&reader, meta, accepted)?;
    let mut remaining = super::super::MAX_FROZEN_PUBLIC_IDENTITY_BYTES;
    [&bundle.declaration, &bundle.head, &bundle.history]
        .into_iter()
        .try_for_each(|value| charge(&mut remaining, value.len()))?;
    let graphs = walk::operations(&reader, accepted, |operation| {
        let encoded_bytes = codec::encoded_len(
            &operation,
            remaining,
            "frozen public identity operation JSON bytes",
        )?;
        charge(&mut remaining, encoded_bytes)?;
        bundle.operations.try_reserve(1).map_err(allocation)?;
        bundle.operations.push(operation);
        Ok(())
    })?;
    let references = walk::graph_objects(&reader, &graphs)?;
    let mut physical_rows = 0usize;
    references.into_values().try_for_each(|reference| {
        let object = physical_object(snapshot, journal, reference, &mut physical_rows, remaining)?;
        let encoded_bytes = codec::encoded_len(
            &object,
            remaining,
            "frozen public identity object JSON bytes",
        )?;
        charge(&mut remaining, encoded_bytes)?;
        bundle.objects.try_reserve(1).map_err(allocation)?;
        bundle.objects.push(object);
        Ok::<(), StoreError>(())
    })?;
    finish(bundle, accepted)
}

pub(super) fn require_snapshot(
    snapshot: &Snapshot,
    accepted: &AcceptedPublicIdentitySnapshot,
) -> StoreResult<()> {
    if accepted.qualification_version()
        != PublicIdentityQualificationVersion::RetainedAuthorityGraphV3
        || accepted.snapshot_sequence() != snapshot.seqno()
    {
        return Err(
            PublicIdentityError::Corrupt("frozen graph requires its V3 accepted snapshot").into(),
        );
    }
    Ok(())
}

pub(super) fn empty_bundle(
    reader: &Live<'_>,
    meta: &Keyspace,
    accepted: &AcceptedPublicIdentitySnapshot,
) -> StoreResult<wire::Bundle> {
    Ok(wire::Bundle {
        version: 2,
        store_identity: accepted.store_identity().clone(),
        revision: reader.revision()?,
        declaration: metadata(reader.snapshot, meta, DECLARATION, 65_536)?,
        head: metadata(reader.snapshot, meta, HEAD, 1024)?,
        history: metadata(
            reader.snapshot,
            meta,
            HISTORY,
            MAX_PUBLIC_IDENTITY_HISTORY_BYTES,
        )?,
        operations: Vec::new(),
        objects: Vec::new(),
    })
}

fn physical_object(
    snapshot: &Snapshot,
    journal: &Keyspace,
    reference: ObjectRefV2,
    physical_rows: &mut usize,
    byte_limit: usize,
) -> StoreResult<wire::Object> {
    let mut rows = BTreeMap::new();
    let mut remaining = byte_limit;
    graph_physical_rows(snapshot, journal, &reference, |key, value| {
        *physical_rows = physical_rows
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        super::super::bound(*physical_rows, wire::MAX_PHYSICAL_ROWS)?;
        let row = wire::PhysicalRow::capture(key, value)?;
        let encoded_bytes = codec::encoded_len(
            &row,
            remaining,
            "frozen public identity physical row JSON bytes",
        )?;
        charge(&mut remaining, encoded_bytes)?;
        if rows.insert(key.to_vec(), row).is_some() {
            return Err(
                PublicIdentityError::Corrupt("duplicate captured graph physical row").into(),
            );
        }
        Ok(())
    })?;
    let rows = rows.into_values().try_fold(Vec::new(), |mut rows, row| {
        rows.try_reserve(1).map_err(allocation)?;
        rows.push(row);
        Ok::<_, StoreError>(rows)
    })?;
    Ok(wire::Object { reference, rows })
}

fn finish(
    bundle: wire::Bundle,
    accepted: &AcceptedPublicIdentitySnapshot,
) -> StoreResult<FrozenPublicIdentityBundleV2> {
    bundle.bounds()?;
    let bytes = codec::encode_with_context(
        &bundle,
        super::super::MAX_FROZEN_PUBLIC_IDENTITY_BYTES,
        "frozen public identity bundle JSON bytes",
    )?;
    let digest = super::super::digest(&bytes);
    let replayed = super::replay::replay(&bytes, &digest)?;
    accepted.require_same_projection(&replayed)?;
    Ok(FrozenPublicIdentityBundleV2 { bytes, digest })
}
fn metadata(
    snapshot: &Snapshot,
    table: &Keyspace,
    key: &str,
    maximum: usize,
) -> StoreResult<String> {
    text(
        codec::load(snapshot, table, key, maximum)?
            .ok_or(PublicIdentityError::Corrupt(
                "missing frozen graph metadata",
            ))?
            .to_vec(),
    )
}
fn text(bytes: Vec<u8>) -> StoreResult<String> {
    String::from_utf8(bytes)
        .map_err(|_| PublicIdentityError::Corrupt("frozen graph text encoding").into())
}
fn charge(remaining: &mut usize, bytes: usize) -> StoreResult<()> {
    *remaining = remaining
        .checked_sub(bytes)
        .ok_or(StoreError::BudgetExceeded {
            what: "frozen graph bundle bytes",
            needed: u64::try_from(bytes).map_err(|_| StoreError::CounterOverflow)?,
            available: u64::try_from(*remaining).map_err(|_| StoreError::CounterOverflow)?,
        })?;
    Ok(())
}
fn allocation(error: std::collections::TryReserveError) -> StoreError {
    StoreError::Refused {
        detail: format!("frozen graph capture allocation: {error}"),
    }
}
