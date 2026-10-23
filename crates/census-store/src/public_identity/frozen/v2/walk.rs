use super::wire;
use crate::public_identity::{
    graph_witness::HeldGraphWitness,
    paged_generation::{capture::verify, CaptureBindingV2, ObjectRefV2},
    retained::{Live, Read, Rows},
    witness, PublicIdentityError,
};
use crate::{AcceptedPublicIdentitySnapshot, StoreError, StoreResult};
use std::collections::BTreeMap;

pub(super) type Graphs = BTreeMap<String, (CaptureBindingV2, ObjectRefV2)>;

pub(super) fn operations(
    reader: &impl Read,
    accepted: &AcceptedPublicIdentitySnapshot,
    mut visit: impl FnMut(wire::Operation) -> StoreResult<()>,
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
            visit(operation)?;
            Ok(graphs)
        })
}

pub(super) fn graph_objects(
    reader: &Live<'_>,
    graphs: &Graphs,
) -> StoreResult<BTreeMap<String, ObjectRefV2>> {
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

fn row(reader: &impl Read, rows: Rows, key: &str) -> StoreResult<String> {
    text(reader.row(rows, key)?.ok_or(PublicIdentityError::Corrupt(
        "missing frozen graph operation row",
    ))?)
}

fn text(bytes: Vec<u8>) -> StoreResult<String> {
    String::from_utf8(bytes)
        .map_err(|_| PublicIdentityError::Corrupt("frozen graph text encoding").into())
}
