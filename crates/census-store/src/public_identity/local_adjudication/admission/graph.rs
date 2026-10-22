mod builder;
mod persist;
mod retained;
mod validate;
pub(in crate::public_identity) use super::graph_wire::GraphReservation;
pub(in crate::public_identity) use super::graph_wire::MAX_METADATA_BYTES;
use super::graph_wire::{Descriptor, Purpose, Reference, MAX_DESCRIPTORS, MAX_PHYSICAL_ROWS};
use super::{add, codec, invalid, ContextCharge, Permit};
use crate::public_identity::paged_generation::{
    admission as bridge, CaptureBindingV2, DigestV2, ObjectRefV2, RawGenerationOperationIdV2,
    RawGenerationSealV2,
};
use crate::{Store, StoreError, StoreResult};
pub(in crate::public_identity) use persist::{load_graph, reserve_graph};

pub(in crate::public_identity) struct GraphPlanBuilder {
    capture: CaptureBindingV2,
    descriptors: Vec<Descriptor>,
    scopes: Vec<Option<crate::public_identity::paged_generation::AuthorityTableV2>>,
    physical_rows: usize,
    metadata_bytes: usize,
}

pub(in crate::public_identity) struct GraphPlan {
    capture: CaptureBindingV2,
    seal: ObjectRefV2,
    descriptors: Vec<Descriptor>,
}

impl GraphReservation {
    pub(in crate::public_identity) fn seal(&self) -> &ObjectRefV2 {
        match &self.purpose {
            Purpose::RawGeneration { seal, .. } => seal,
        }
    }
    pub(in crate::public_identity) fn capture(&self) -> &CaptureBindingV2 {
        match &self.purpose {
            Purpose::RawGeneration { capture, .. } => capture,
        }
    }
    fn operation(&self) -> &RawGenerationOperationIdV2 {
        match &self.purpose {
            Purpose::RawGeneration { operation, .. } => operation,
        }
    }
    pub(in crate::public_identity) fn is_complete(&self, store: &Store) -> StoreResult<bool> {
        validate::validate(self, &retained_identity(store)?)?;
        audit_retained(store, self)
    }
    pub(in crate::public_identity) fn permit_for<'s>(
        &self,
        store: &'s Store,
        root: &str,
    ) -> StoreResult<Permit<'s>> {
        validate::validate(self, &retained_identity(store)?)?;
        let descriptor = self
            .descriptors
            .iter()
            .find(|entry| entry.object.root().as_str() == root)
            .ok_or_else(invalid)?;
        descriptor.physical.audit(store)?;
        crate::journal::context::verify_retained_charge(store, &descriptor.incremental)?;
        require_children_ready(store, descriptor, &self.descriptors)?;
        if root == self.seal().root().as_str() {
            for child in self
                .descriptors
                .iter()
                .filter(|entry| entry.object.root().as_str() != root)
            {
                child.physical.audit_ready(store)?;
            }
        }
        Ok(Permit {
            store,
            root: root.to_owned(),
            ready_required: descriptor.incremental.roots == 0,
        })
    }
    pub(in crate::public_identity) fn require_plan(&self, plan: &GraphPlan) -> StoreResult<()> {
        if self.capture() != &plan.capture
            || self.seal() != &plan.seal
            || self.descriptors.len() != plan.descriptors.len()
        {
            return Err(invalid());
        }
        for (original, current) in self.descriptors.iter().zip(&plan.descriptors) {
            if original.object != current.object
                || original.physical != current.physical
                || codec::encode(&original.children, MAX_METADATA_BYTES)?
                    != codec::encode(&current.children, MAX_METADATA_BYTES)?
                || original.incremental.root != current.incremental.root
                || current.incremental.bytes > original.incremental.bytes
                || current.incremental.roots > original.incremental.roots
            {
                return Err(invalid());
            }
        }
        Ok(())
    }
}

pub(super) fn audit(
    store: &Store,
    key: &str,
    raw: &[u8],
    identity: &census_domain::model::StoreIdentity,
) -> StoreResult<(u64, u64)> {
    let reservation: GraphReservation = codec::decode(key, raw)?;
    validate::validate(&reservation, identity)?;
    if key != super::key(reservation.operation().as_str()) {
        return Err(invalid());
    }
    audit_retained(store, &reservation)?;
    totals(&reservation, key.len(), raw.len())
}

fn totals(
    reservation: &GraphReservation,
    key_bytes: usize,
    value_bytes: usize,
) -> StoreResult<(u64, u64)> {
    let metadata = key_bytes
        .checked_add(value_bytes)
        .ok_or(StoreError::CounterOverflow)?;
    if metadata > MAX_METADATA_BYTES {
        return Err(invalid());
    }
    reservation.descriptors.iter().try_fold(
        (
            u64::try_from(metadata).map_err(|_| StoreError::CounterOverflow)?,
            0,
        ),
        |(bytes, roots), entry| {
            Ok((
                add(bytes, entry.incremental.bytes)?,
                add(roots, entry.incremental.roots)?,
            ))
        },
    )
}

#[cfg(test)]
mod tests;

fn audit_retained(store: &Store, reservation: &GraphReservation) -> StoreResult<bool> {
    use fjall::Readable;
    let snapshot = store.db.snapshot();
    let seal_key = crate::journal::object_manifest_key(reservation.seal().root().as_str())?;
    let complete = snapshot
        .contains_key(&store.journal, &seal_key)
        .map_err(|source| StoreError::Read { source })?;
    for entry in &reservation.descriptors {
        entry.physical.audit(store)?;
        crate::journal::context::verify_retained_charge(store, &entry.incremental)?;
        if complete {
            let manifest = crate::journal::object_manifest_key(entry.object.root().as_str())?;
            if !snapshot
                .contains_key(&store.journal, &manifest)
                .map_err(|source| StoreError::Read { source })?
            {
                return Err(invalid());
            }
        }
    }
    if complete {
        retained::verify(store, reservation)?;
    }
    Ok(complete)
}

fn require_children_ready(
    store: &Store,
    descriptor: &Descriptor,
    descriptors: &[Descriptor],
) -> StoreResult<()> {
    use fjall::Readable;
    let snapshot = store.db.snapshot();
    for child in &descriptor.children {
        let retained = descriptors
            .iter()
            .find(|entry| entry.object == child.0)
            .ok_or_else(invalid)?;
        retained.physical.audit(store)?;
        let manifest = crate::journal::object_manifest_key(child.0.root().as_str())?;
        if !snapshot
            .contains_key(&store.journal, &manifest)
            .map_err(|source| StoreError::Read { source })?
        {
            return Err(invalid());
        }
    }
    Ok(())
}

fn retained_identity(store: &Store) -> StoreResult<census_domain::model::StoreIdentity> {
    match crate::lineage::read(&store.db.snapshot(), &store.journal)? {
        crate::lineage::RetainedIdentity::Present(identity) => Ok(identity),
        crate::lineage::RetainedIdentity::Absent => Err(invalid()),
    }
}
