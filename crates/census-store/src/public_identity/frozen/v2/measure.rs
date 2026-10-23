use super::{capture, walk, wire};
use crate::public_identity::{
    codec, paged_generation::graph_physical_rows, retained::Live, PublicIdentityError,
};
use crate::{AcceptedPublicIdentitySnapshot, StoreError, StoreResult};
use census_domain::{model::StoreIdentity, public_identity_history::RegistryHead};
use fjall::{Keyspace, Snapshot};
use serde::Serialize;
use std::collections::BTreeSet;

mod accounting;
mod normal;
use accounting::{checked_sum, ByteBudget, JsonArrayBytes};
use normal::ComponentBudget;

pub const MAX_FROZEN_PUBLIC_IDENTITY_DEMAND_JSON_BYTES: usize = 16 * 1024 * 1024 * 1024;
pub(super) struct JsonByteLimits {
    pub(super) diagnostic: usize,
    pub(super) normal: usize,
}

const CONTEXT: &str = "frozen identity diagnostic canonical JSON bytes";

#[derive(Serialize)]
pub struct FrozenPublicIdentityDemandV2 {
    store_identity: StoreIdentity,
    snapshot_sequence: u64,
    accepted_head: RegistryHead,
    diagnostic_maximum_json_bytes: usize,
    canonical_json_bytes: usize,
    empty_bundle_json_bytes: usize,
    operation_array_content_json_bytes: usize,
    object_array_content_json_bytes: usize,
    operations: usize,
    objects: usize,
    physical_rows: usize,
    normal_component_budget: ComponentBudget,
    normal_json_byte_limit: usize,
    fits_normal_json_byte_limit: bool,
}

impl FrozenPublicIdentityDemandV2 {
    pub fn canonical_json_bytes(&self) -> usize {
        self.canonical_json_bytes
    }
}

pub(crate) fn measure(
    snapshot: &Snapshot,
    entities: &Keyspace,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    accepted: &AcceptedPublicIdentitySnapshot,
) -> StoreResult<FrozenPublicIdentityDemandV2> {
    measure_with_limits(
        snapshot,
        entities,
        meta,
        journal,
        receipts,
        accepted,
        JsonByteLimits {
            diagnostic: MAX_FROZEN_PUBLIC_IDENTITY_DEMAND_JSON_BYTES,
            normal: super::super::MAX_FROZEN_PUBLIC_IDENTITY_BYTES,
        },
    )
}

pub(super) fn measure_with_limits(
    snapshot: &Snapshot,
    entities: &Keyspace,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    accepted: &AcceptedPublicIdentitySnapshot,
    limits: JsonByteLimits,
) -> StoreResult<FrozenPublicIdentityDemandV2> {
    capture::require_snapshot(snapshot, accepted)?;
    let reader = Live::new(snapshot, entities, meta, journal, receipts)?;
    let bundle = capture::empty_bundle(&reader, meta, accepted)?;
    bundle.bounds()?;
    super::super::bound(
        accepted.history_records().len(),
        crate::public_identity::REPLAY_LIMITS.records,
    )?;
    let framing = codec::encoded_len(&bundle, limits.diagnostic, CONTEXT)?;
    let mut total = ByteBudget::new(limits.diagnostic);
    total.add(framing)?;
    let mut normal = ComponentBudget::new(limits.normal);
    [&bundle.declaration, &bundle.head, &bundle.history]
        .into_iter()
        .for_each(|value| normal.charge(value.len(), "raw_metadata"));
    let (graphs, operations) = operations_demand(&reader, accepted, &mut total, &mut normal)?;
    let (objects, physical_rows) = objects_demand(&reader, &graphs, &mut total, &mut normal)?;
    Ok(FrozenPublicIdentityDemandV2 {
        store_identity: accepted.store_identity().clone(),
        snapshot_sequence: snapshot.seqno(),
        accepted_head: accepted.head().clone(),
        diagnostic_maximum_json_bytes: limits.diagnostic,
        canonical_json_bytes: total.used(),
        empty_bundle_json_bytes: framing,
        operation_array_content_json_bytes: operations.content(),
        object_array_content_json_bytes: objects.content(),
        operations: operations.items(),
        objects: objects.items(),
        physical_rows,
        normal_component_budget: normal,
        normal_json_byte_limit: limits.normal,
        fits_normal_json_byte_limit: total.used() <= limits.normal,
    })
}

fn operations_demand(
    reader: &Live<'_>,
    accepted: &AcceptedPublicIdentitySnapshot,
    total: &mut ByteBudget,
    normal: &mut ComponentBudget,
) -> StoreResult<(walk::Graphs, JsonArrayBytes)> {
    let mut operations = JsonArrayBytes::default();
    let graphs = walk::operations(reader, accepted, |operation| {
        let bytes = codec::encoded_len(&operation, total.remaining()?, CONTEXT)?;
        add_item(total, &mut operations, bytes)?;
        normal.charge(bytes, "operation_json");
        Ok(())
    })?;
    Ok((graphs, operations))
}

fn objects_demand(
    reader: &Live<'_>,
    graphs: &walk::Graphs,
    total: &mut ByteBudget,
    normal: &mut ComponentBudget,
) -> StoreResult<(JsonArrayBytes, usize)> {
    let references = walk::graph_objects(reader, graphs)?;
    let mut objects = JsonArrayBytes::default();
    let mut physical_rows = 0;
    references.into_values().try_for_each(|reference| {
        let separator = usize::from(objects.items() != 0);
        total.add(separator)?;
        let object = wire::Object {
            reference,
            rows: Vec::new(),
        };
        let object_framing = codec::encoded_len(&object, total.remaining()?, CONTEXT)?;
        total.add(object_framing)?;
        let mut local = normal.clone();
        let rows = physical_rows_demand(
            reader.snapshot,
            reader.journal,
            &object,
            &mut physical_rows,
            total,
            &mut local,
        )?;
        let bytes = checked_sum(object_framing, rows.content())?;
        normal.finish_object(local, bytes);
        objects.push(bytes)?;
        Ok::<(), StoreError>(())
    })?;
    Ok((objects, physical_rows))
}

fn physical_rows_demand(
    snapshot: &Snapshot,
    journal: &Keyspace,
    object: &wire::Object,
    physical_rows: &mut usize,
    total: &mut ByteBudget,
    normal: &mut ComponentBudget,
) -> StoreResult<JsonArrayBytes> {
    let mut keys = BTreeSet::new();
    let mut rows = JsonArrayBytes::default();
    graph_physical_rows(snapshot, journal, &object.reference, |key, value| {
        *physical_rows = physical_rows
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        super::super::bound(*physical_rows, wire::MAX_PHYSICAL_ROWS)?;
        let row = wire::PhysicalRow::capture(key, value)?;
        let bytes = codec::encoded_len(&row, total.remaining()?, CONTEXT)?;
        add_item(total, &mut rows, bytes)?;
        normal.charge(bytes, "physical_row_json");
        if !keys.insert(key.to_vec()) {
            return Err(
                PublicIdentityError::Corrupt("duplicate captured graph physical row").into(),
            );
        }
        Ok(())
    })?;
    Ok(rows)
}

fn add_item(total: &mut ByteBudget, array: &mut JsonArrayBytes, bytes: usize) -> StoreResult<()> {
    let additional = checked_sum(bytes, usize::from(array.items() != 0))?;
    total.add(additional)?;
    array.push(bytes)
}
