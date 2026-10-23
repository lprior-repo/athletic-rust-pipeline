use super::wire;
use crate::public_identity::{
    graph_witness::HeldGraphWitness,
    paged_generation::{capture::verify, CaptureBindingV2, ObjectRefV2, RawObjectSource},
    retained::{Read, Revision, Rows},
    witness, PublicIdentityError,
};
use crate::{StoreError, StoreResult};
use std::collections::BTreeMap;

pub(super) struct Retained<'a> {
    revision: Revision,
    witnesses: BTreeMap<String, &'a str>,
    receipts: BTreeMap<String, &'a str>,
    decisions: BTreeMap<String, &'a str>,
    pub(super) graphs: BTreeMap<String, (CaptureBindingV2, ObjectRefV2)>,
    pub(super) bindings: Vec<CaptureBindingV2>,
    pub(super) physical: BTreeMap<Vec<u8>, Vec<u8>>,
    objects: BTreeMap<String, ObjectRefV2>,
}

impl<'a> Retained<'a> {
    pub(super) fn new(
        revision: Revision,
        operations: &'a [wire::Operation],
        objects: Vec<wire::Object>,
    ) -> StoreResult<Self> {
        let mut reader = Self {
            revision,
            witnesses: BTreeMap::new(),
            receipts: BTreeMap::new(),
            decisions: BTreeMap::new(),
            graphs: BTreeMap::new(),
            bindings: Vec::new(),
            physical: BTreeMap::new(),
            objects: BTreeMap::new(),
        };
        operations
            .iter()
            .enumerate()
            .try_for_each(|(index, operation)| reader.operation(index, operation))?;
        objects
            .into_iter()
            .try_for_each(|object| reader.object(object))?;
        Ok(reader)
    }

    fn operation(&mut self, index: usize, operation: &'a wire::Operation) -> StoreResult<()> {
        let sequence = u64::try_from(index)
            .map_err(|_| StoreError::CounterOverflow)?
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        if sequence != operation.sequence {
            return Err(PublicIdentityError::Corrupt("frozen graph operation sequence").into());
        }
        let held = HeldGraphWitness::decode("frozen graph witness", operation.witness.as_bytes())?;
        insert(
            &mut self.witnesses,
            witness::key(sequence),
            &operation.witness,
        )?;
        insert(
            &mut self.receipts,
            witness::receipt_key(held.operation()),
            &operation.receipt,
        )?;
        match (held.decision_id(), operation.decision.as_deref()) {
            (Some(id), Some(bytes)) => insert(&mut self.decisions, id.to_owned(), bytes)?,
            (None, None) => (),
            _ => {
                return Err(
                    PublicIdentityError::Corrupt("frozen graph decision material kind").into(),
                )
            }
        }
        let graph = held.graph();
        let key = graph.seal().root().as_str().to_owned();
        let value = (graph.capture().clone(), graph.seal().clone());
        match self.graphs.get(&key) {
            Some(prior) if prior != &value => {
                return Err(PublicIdentityError::Corrupt("conflicting frozen graph root").into());
            }
            Some(_) => (),
            None => {
                self.graphs.insert(key, value);
            }
        }
        if !self.bindings.contains(graph.capture()) {
            self.bindings.try_reserve(1).map_err(allocation)?;
            self.bindings.push(graph.capture().clone());
        }
        Ok(())
    }

    fn object(&mut self, object: wire::Object) -> StoreResult<()> {
        let root = object.reference.root().as_str();
        if self
            .objects
            .insert(root.to_owned(), object.reference.clone())
            .is_some()
        {
            return Err(PublicIdentityError::Corrupt("duplicate frozen graph object").into());
        }
        object
            .rows
            .into_iter()
            .try_fold(None, |previous: Option<Vec<u8>>, row| {
                let (key, value) = row.decode()?;
                if previous.as_ref().is_some_and(|prior| prior >= &key) || !owns(root, &key)? {
                    return Err(PublicIdentityError::Corrupt(
                        "frozen graph physical row owner or order",
                    )
                    .into());
                }
                let previous = key.clone();
                if self.physical.insert(key, value).is_some() {
                    return Err(PublicIdentityError::Corrupt(
                        "duplicate frozen graph physical row",
                    )
                    .into());
                }
                Ok::<_, StoreError>(Some(previous))
            })?;
        Ok(())
    }

    pub(super) fn verify_inventory(&self, source: &impl RawObjectSource) -> StoreResult<()> {
        let mut reached = BTreeMap::new();
        self.graphs.values().try_for_each(|(binding, seal)| {
            verify::verify(source, binding, seal, |_, object| {
                let key = object.root().as_str();
                match reached.get(key) {
                    Some(prior) if prior != object => {
                        return Err(PublicIdentityError::Corrupt(
                            "conflicting frozen graph reference",
                        )
                        .into());
                    }
                    Some(_) => (),
                    None => {
                        let next = reached
                            .len()
                            .checked_add(1)
                            .ok_or(StoreError::CounterOverflow)?;
                        super::super::bound(next, wire::MAX_OBJECTS)?;
                        reached.insert(key.to_owned(), object.clone());
                    }
                }
                Ok(())
            })?;
            Ok::<(), StoreError>(())
        })?;
        if reached != self.objects {
            return Err(PublicIdentityError::Corrupt("frozen graph reachable inventory").into());
        }
        Ok(())
    }
}

impl Read for Retained<'_> {
    fn row(&self, rows: Rows, key: &str) -> StoreResult<Option<Vec<u8>>> {
        let source = match rows {
            Rows::Witness => &self.witnesses,
            Rows::Receipt => &self.receipts,
            Rows::Decision => &self.decisions,
        };
        Ok(source.get(key).map(|value| value.as_bytes().to_vec()))
    }
    fn revision(&self) -> StoreResult<Revision> {
        Ok(self.revision)
    }
}

fn insert<'a>(map: &mut BTreeMap<String, &'a str>, key: String, bytes: &'a str) -> StoreResult<()> {
    if map.insert(key, bytes).is_some() {
        return Err(
            PublicIdentityError::Corrupt("duplicate frozen graph operation material").into(),
        );
    }
    Ok(())
}
fn owns(root: &str, key: &[u8]) -> StoreResult<bool> {
    Ok(key == crate::journal::object_manifest_key(root)?
        || crate::journal::object_ordinal(key, &crate::journal::object_chunk_prefix(root)?)
            .is_some()
        || crate::journal::object_ordinal(key, &crate::journal::object_index_prefix(root)?)
            .is_some())
}
fn allocation(error: std::collections::TryReserveError) -> StoreError {
    StoreError::Refused {
        detail: format!("frozen graph inventory allocation: {error}"),
    }
}
