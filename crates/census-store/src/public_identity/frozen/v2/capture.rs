use super::{wire, FrozenPublicIdentityBundleV2};
use crate::public_identity::{
    codec,
    graph_witness::HeldGraphWitness,
    paged_generation::{capture::verify, graph_physical_rows, CaptureBindingV2, ObjectRefV2},
    retained::{Live, Read, Rows},
    witness, PublicIdentityError, DECLARATION, HEAD, HISTORY, MAX_PUBLIC_IDENTITY_HISTORY_BYTES,
};
use crate::{AcceptedPublicIdentitySnapshot, StoreError, StoreResult};
use census_domain::public_identity_qualification::PublicIdentityQualificationVersion;
use fjall::{Keyspace, Snapshot};
use std::collections::BTreeMap;

type Graphs = BTreeMap<String, (CaptureBindingV2, ObjectRefV2)>;

pub(crate) fn capture(
    snapshot: &Snapshot,
    entities: &Keyspace,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    accepted: &AcceptedPublicIdentitySnapshot,
) -> StoreResult<FrozenPublicIdentityBundleV2> {
    if accepted.qualification_version()
        != PublicIdentityQualificationVersion::RetainedAuthorityGraphV3
        || accepted.snapshot_sequence() != snapshot.seqno()
    {
        return Err(
            PublicIdentityError::Corrupt("frozen graph requires its V3 accepted snapshot").into(),
        );
    }
    let reader = Live {
        snapshot,
        entities,
        meta,
        journal,
        receipts,
    };
    let mut bundle = wire::Bundle {
        version: 2,
        store_identity: accepted.store_identity().clone(),
        revision: reader.revision()?,
        declaration: metadata(snapshot, meta, DECLARATION, 65_536)?,
        head: metadata(snapshot, meta, HEAD, 1024)?,
        history: metadata(snapshot, meta, HISTORY, MAX_PUBLIC_IDENTITY_HISTORY_BYTES)?,
        operations: Vec::new(),
        objects: Vec::new(),
    };
    let mut remaining = super::super::MAX_FROZEN_PUBLIC_IDENTITY_BYTES;
    [&bundle.declaration, &bundle.head, &bundle.history]
        .into_iter()
        .try_for_each(|value| charge(&mut remaining, value.len()))?;
    let graphs = operations(&reader, accepted, &mut bundle, &mut remaining)?;
    let references = graph_objects(&reader, &graphs)?;
    let mut physical_rows = 0usize;
    references.into_values().try_for_each(|reference| {
        let object = physical_object(snapshot, journal, reference, &mut physical_rows, remaining)?;
        let encoded_bytes = codec::encode(&object, remaining)?.len();
        charge(&mut remaining, encoded_bytes)?;
        bundle.objects.try_reserve(1).map_err(allocation)?;
        bundle.objects.push(object);
        Ok::<(), StoreError>(())
    })?;
    finish(bundle, accepted)
}

fn operations(
    reader: &impl Read,
    accepted: &AcceptedPublicIdentitySnapshot,
    bundle: &mut wire::Bundle,
    remaining: &mut usize,
) -> StoreResult<Graphs> {
    accepted
        .history_records()
        .iter()
        .try_fold(BTreeMap::new(), |mut graphs, record| {
            let raw = row(reader, Rows::Witness, &witness::key(record.sequence))?;
            let held = HeldGraphWitness::decode("frozen graph witness", raw.as_bytes())?;
            let operation = wire::Operation {
                sequence: record.sequence,
                receipt: row(
                    reader,
                    Rows::Receipt,
                    &witness::receipt_key(held.operation()),
                )?,
                decision: held
                    .decision_id()
                    .map(|id| row(reader, Rows::Decision, id))
                    .transpose()?,
                witness: raw,
            };
            let graph = held.graph();
            let key = graph.seal().root().as_str().to_owned();
            let value = (graph.capture().clone(), graph.seal().clone());
            if graphs.get(&key).is_some_and(|prior| prior != &value) {
                return Err(
                    PublicIdentityError::Corrupt("frozen graph conflicting root binding").into(),
                );
            }
            graphs.insert(key, value);
            let encoded_bytes = codec::encode(&operation, *remaining)?.len();
            charge(remaining, encoded_bytes)?;
            bundle.operations.try_reserve(1).map_err(allocation)?;
            bundle.operations.push(operation);
            Ok(graphs)
        })
}

fn graph_objects(reader: &Live<'_>, graphs: &Graphs) -> StoreResult<BTreeMap<String, ObjectRefV2>> {
    let mut objects = BTreeMap::new();
    graphs.values().try_for_each(|(binding, seal)| {
        verify::verify(reader, binding, seal, |_, object| {
            let key = object.root().as_str();
            match objects.get(key) {
                Some(prior) if prior != object => {
                    return Err(PublicIdentityError::Corrupt(
                        "frozen graph conflicting object reference",
                    )
                    .into());
                }
                Some(_) => (),
                None => {
                    let next = objects
                        .len()
                        .checked_add(1)
                        .ok_or(StoreError::CounterOverflow)?;
                    super::super::bound(next, wire::MAX_OBJECTS)?;
                    objects.insert(key.to_owned(), object.clone());
                }
            }
            Ok(())
        })?;
        Ok::<(), StoreError>(())
    })?;
    Ok(objects)
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
        let encoded_bytes = codec::encode(&row, remaining)?.len();
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
    let bytes = codec::encode(&bundle, super::super::MAX_FROZEN_PUBLIC_IDENTITY_BYTES)?;
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
fn row(reader: &impl Read, rows: Rows, key: &str) -> StoreResult<String> {
    text(reader.row(rows, key)?.ok_or(PublicIdentityError::Corrupt(
        "missing frozen graph operation row",
    ))?)
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
