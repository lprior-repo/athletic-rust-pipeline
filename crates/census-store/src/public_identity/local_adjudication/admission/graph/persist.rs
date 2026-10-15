use super::*;
use fjall::PersistMode;

pub(in crate::public_identity) fn reserve_graph(
    store: &Store,
    operation: &RawGenerationOperationIdV2,
    plan: GraphPlan,
) -> StoreResult<GraphReservation> {
    let identity = bridge::capture_store(&plan.capture);
    let mut header = super::super::header(store, identity)?;
    let key = super::super::key(operation.as_str());
    if codec::load(
        &store.db.snapshot(),
        &store.journal,
        &key,
        MAX_METADATA_BYTES,
    )?
    .is_some()
    {
        return Err(invalid());
    }
    let reservation = GraphReservation {
        version: 3,
        purpose: Purpose::RawGeneration {
            operation: operation.clone(),
            capture: plan.capture,
            seal: plan.seal,
        },
        descriptors: plan.descriptors,
    };
    validate::validate(&reservation, &header.store)?;
    for entry in &reservation.descriptors {
        entry.physical.audit(store)?;
        crate::journal::context::verify_retained_charge(store, &entry.incremental)?;
    }
    let value_limit = MAX_METADATA_BYTES
        .checked_sub(key.len())
        .ok_or(StoreError::CounterOverflow)?;
    let raw = codec::encode(&reservation, value_limit)?;
    let (bytes, roots) = totals(&reservation, key.len(), raw.len())?;
    header.bytes = add(header.bytes, bytes)?;
    header.roots = add(header.roots, roots)?;
    header.reservations = add(header.reservations, 1)?;
    super::super::require_capacity(header.bytes, header.roots, header.reservations)?;
    let encoded = codec::encode(&header, super::super::header_payload_limit()?)?;
    let mut batch = store.db.batch();
    batch.insert(&store.journal, key.as_bytes(), raw);
    batch.insert(&store.journal, super::super::HEADER.as_bytes(), encoded);
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })?;
    Ok(reservation)
}

pub(in crate::public_identity) fn load_graph(
    store: &Store,
    identity: &census_domain::model::StoreIdentity,
    operation: &RawGenerationOperationIdV2,
) -> StoreResult<Option<GraphReservation>> {
    super::super::header(store, identity)?;
    let key = super::super::key(operation.as_str());
    let Some(raw) = codec::load(
        &store.db.snapshot(),
        &store.journal,
        &key,
        MAX_METADATA_BYTES,
    )?
    else {
        return Ok(None);
    };
    let reservation: GraphReservation = codec::decode(&key, &raw)?;
    validate::validate(&reservation, identity)?;
    if reservation.operation() != operation {
        return Err(invalid());
    }
    Ok(Some(reservation))
}
