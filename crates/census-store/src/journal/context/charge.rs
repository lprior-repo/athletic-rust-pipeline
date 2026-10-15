use super::physical_ref::{ContextPhysicalPlan, ContextPhysicalRef, PhysicalRows};
use super::{persist, refused, run_pass, wire, ContextRecordSink, IndexEntry, ObjectPlan, Pass};
use crate::journal::{object_chunk_key, object_index_key, object_manifest_key};
use crate::{Store, StoreError, StoreResult};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct ContextCharge {
    pub(crate) root: String,
    pub(crate) bytes: u64,
    pub(crate) roots: u64,
}

pub(super) struct ChargeTarget<'a> {
    store: &'a Store,
    root: &'a str,
    ready: bool,
    bytes: &'a mut u64,
    physical: &'a mut PhysicalRows,
}

impl ChargeTarget<'_> {
    fn row(&mut self, key: &[u8], value: &[u8]) -> StoreResult<()> {
        match persist::read_row(self.store, key)? {
            Some(existing) => persist::require_equal(key, &existing, value)?,
            None if self.ready => return Err(refused("ready charged object has missing row")),
            None => (),
        }
        let size = key
            .len()
            .checked_add(value.len())
            .ok_or(StoreError::CounterOverflow)?;
        *self.bytes = self
            .bytes
            .checked_add(u64::try_from(size).map_err(|_| StoreError::CounterOverflow)?)
            .ok_or(StoreError::CounterOverflow)?;
        Ok(())
    }

    pub(super) fn chunk(&mut self, index: usize, bytes: &[u8], digest: &str) -> StoreResult<()> {
        let value = wire::chunk(self.root, index, bytes, digest)?;
        self.row(&object_chunk_key(self.root, index)?, &value)?;
        self.physical.chunk(index, &value)
    }

    pub(super) fn page(&mut self, page: usize, entries: &[IndexEntry]) -> StoreResult<()> {
        let value = wire::page(self.root, page, entries)?;
        self.row(&object_index_key(self.root, page)?, &value)?;
        self.physical.page(page, &value)
    }
}

#[cfg(test)]
pub(crate) fn charge_context_object<F>(store: &Store, emit: F) -> StoreResult<ContextCharge>
where
    F: Fn(&mut ContextRecordSink<'_>) -> StoreResult<()>,
{
    Ok(measure_context_physical(store, emit)?
        .incremental_charge()
        .clone())
}

pub(crate) fn measure_context_physical<F>(
    store: &Store,
    emit: F,
) -> StoreResult<ContextPhysicalPlan>
where
    F: Fn(&mut ContextRecordSink<'_>) -> StoreResult<()>,
{
    let plan = run_pass(Pass::Measure, &emit)?;
    charge_plan(store, plan, emit)
}

fn charge_plan<F>(store: &Store, plan: ObjectPlan, emit: F) -> StoreResult<ContextPhysicalPlan>
where
    F: Fn(&mut ContextRecordSink<'_>) -> StoreResult<()>,
{
    persist::verify_namespace(store, &plan)?;
    let key = object_manifest_key(&plan.staged.object)?;
    let value = wire::manifest(&plan)?;
    let existing = persist::read_row(store, &key)?;
    if let Some(raw) = &existing {
        persist::require_equal(&key, raw, &value)?;
    }
    let ready = existing.is_some();
    let mut bytes = 0;
    let mut physical = PhysicalRows::new(&plan, &value)?;
    let mut target = ChargeTarget {
        store,
        root: &plan.staged.object,
        ready,
        bytes: &mut bytes,
        physical: &mut physical,
    };
    target.row(&key, &value)?;
    let observed = run_pass(Pass::Charge(target), &emit)?;
    if observed != plan {
        return Err(refused("charged context changed between passes"));
    }
    let charge = ContextCharge {
        root: plan.staged.object.clone(),
        bytes: if ready { 0 } else { bytes },
        roots: u64::from(!ready),
    };
    let physical = ContextPhysicalRef::from_plan(plan, physical, bytes)?;
    let result = ContextPhysicalPlan::new(physical, charge);
    result.audit(store)?;
    Ok(result)
}

pub(crate) fn verify_retained_charge(store: &Store, charge: &ContextCharge) -> StoreResult<()> {
    use fjall::Readable;
    let manifest = object_manifest_key(&charge.root)?;
    let ready = persist::read_row(store, &manifest)?;
    if charge.roots == 0 {
        return ready
            .map(|_| ())
            .ok_or_else(|| refused("uncharged reserved root is not ready"));
    }
    let mut total = match ready {
        Some(raw) => manifest
            .len()
            .checked_add(raw.len())
            .ok_or(StoreError::CounterOverflow)?,
        None => 0,
    };
    for (prefix, limit) in [
        (
            crate::journal::object_chunk_prefix(&charge.root)?,
            super::CONTEXT_CHUNKS,
        ),
        (
            crate::journal::object_index_prefix(&charge.root)?,
            super::CONTEXT_PAGES,
        ),
    ] {
        for (index, guard) in store
            .db
            .snapshot()
            .prefix(&store.journal, &prefix)
            .take(limit + 1)
            .enumerate()
        {
            if index >= limit {
                return Err(refused("reserved physical row count exceeded"));
            }
            let key = guard.key().map_err(|source| StoreError::Read { source })?;
            let raw = persist::read_row(store, &key)?
                .ok_or_else(|| refused("reserved row disappeared"))?;
            total = total
                .checked_add(key.len())
                .and_then(|sum| sum.checked_add(raw.len()))
                .ok_or(StoreError::CounterOverflow)?;
            if u64::try_from(total).map_err(|_| StoreError::CounterOverflow)? > charge.bytes {
                return Err(refused("reserved physical bytes exceed charge"));
            }
        }
    }
    if u64::try_from(total).map_err(|_| StoreError::CounterOverflow)? > charge.bytes {
        return Err(refused("reserved manifest bytes exceed charge"));
    }
    Ok(())
}
