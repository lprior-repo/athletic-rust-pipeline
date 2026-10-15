use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{Store, StoreError, StoreResult};

mod charge;
mod encode;
#[cfg(test)]
pub(crate) use charge::charge_context_object;
pub(crate) use charge::{measure_context_physical, verify_retained_charge, ContextCharge};
mod persist;
mod physical_ref;
mod stream;
mod wire;

pub(crate) use physical_ref::{ContextPhysicalPlan, ContextPhysicalRef};
pub(crate) use stream::ContextRecordSink;

pub(crate) const CONTEXT_RECORD_BYTES: usize = 1024 * 1024;
pub(crate) const CONTEXT_TOTAL_BYTES: usize = 64 * 1024 * 1024;
pub(crate) const CONTEXT_RECORDS: usize = 32_769;
const CONTEXT_CHUNKS: usize = CONTEXT_TOTAL_BYTES / super::OBJECT_CHUNK_BYTES;
const CONTEXT_PAGES: usize = CONTEXT_RECORDS.div_ceil(super::OBJECT_INDEX_PAGE_ENTRIES);

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct StagedContextObject {
    object: String,
    total_bytes: usize,
    records: usize,
    chunks: usize,
}

impl StagedContextObject {
    #[cfg(test)]
    pub(crate) fn object_id(&self) -> &str {
        &self.object
    }

    #[cfg(test)]
    pub(crate) fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    #[cfg(test)]
    pub(crate) fn record_count(&self) -> usize {
        self.records
    }

    #[cfg(test)]
    pub(crate) fn chunk_count(&self) -> usize {
        self.chunks
    }
}

#[derive(Debug, PartialEq, Eq)]
struct ObjectPlan {
    staged: StagedContextObject,
    chunk_digests: Vec<String>,
    page_digests: Vec<String>,
}

#[derive(Clone, Copy, Serialize)]
struct IndexEntry {
    sequence: usize,
    offset: usize,
    length: usize,
}

enum Pass<'a> {
    Measure,
    Charge(charge::ChargeTarget<'a>),
    Stage(persist::StageTarget<'a>),
}

pub(crate) fn measure_context_object<F>(emit: F) -> StoreResult<String>
where
    F: Fn(&mut ContextRecordSink<'_>) -> StoreResult<()>,
{
    Ok(run_pass(Pass::Measure, &emit)?.staged.object)
}

#[cfg(test)]
pub(crate) fn stage_context_object<F>(store: &Store, emit: F) -> StoreResult<StagedContextObject>
where
    F: Fn(&mut ContextRecordSink<'_>) -> StoreResult<()>,
{
    let plan = run_pass(Pass::Measure, &emit)?;
    stage_plan(store, plan, &emit)
}

pub(crate) fn stage_reserved_context<F>(
    store: &Store,
    permit: &crate::public_identity::ManualContextPermit<'_>,
    emit: F,
) -> StoreResult<StagedContextObject>
where
    F: Fn(&mut ContextRecordSink<'_>) -> StoreResult<()>,
{
    let root = permit.root_for(store)?;
    let plan = run_pass(Pass::Measure, &emit)?;
    if plan.staged.object != root {
        return Err(refused("reserved context root changed"));
    }
    stage_plan(store, plan, &emit)
}

fn stage_plan<F>(store: &Store, plan: ObjectPlan, emit: &F) -> StoreResult<StagedContextObject>
where
    F: Fn(&mut ContextRecordSink<'_>) -> StoreResult<()>,
{
    let mut physical = persist::PhysicalDigests::new()?;
    let target = persist::StageTarget::open(store, &plan, &mut physical)?;
    let presence = target.presence();
    let observed = run_pass(Pass::Stage(target), &emit)?;
    if observed != plan {
        return Err(refused("callback changed between context-object passes"));
    }
    persist::publish_manifest(store, &plan, &physical, presence)?;
    Ok(plan.staged)
}

fn run_pass<F>(pass: Pass<'_>, emit: &F) -> StoreResult<ObjectPlan>
where
    F: Fn(&mut ContextRecordSink<'_>) -> StoreResult<()>,
{
    let mut sink = ContextRecordSink::new(pass)?;
    emit(&mut sink)?;
    sink.finish()
}

fn index_digest(entries: &[IndexEntry]) -> StoreResult<String> {
    let hash = entries.iter().try_fold(Sha256::new(), |hash, entry| {
        [entry.sequence, entry.offset, entry.length]
            .into_iter()
            .try_fold(hash, |mut hash, number| {
                let number = u64::try_from(number).map_err(|_| StoreError::CounterOverflow)?;
                hash.update(number.to_be_bytes());
                Ok::<_, StoreError>(hash)
            })
    })?;
    Ok(format!("{:x}", hash.finalize()))
}

fn bounded(what: &'static str, needed: usize, available: usize) -> StoreResult<()> {
    if needed <= available {
        return Ok(());
    }
    Err(StoreError::BudgetExceeded {
        what,
        needed: u64::try_from(needed).map_err(|_| StoreError::CounterOverflow)?,
        available: u64::try_from(available).map_err(|_| StoreError::CounterOverflow)?,
    })
}

fn reserved<T>(capacity: usize) -> StoreResult<Vec<T>> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(capacity)
        .map_err(|source| refused(&format!("cannot reserve bounded context buffer: {source}")))?;
    Ok(values)
}

fn refused(detail: &str) -> StoreError {
    StoreError::Refused {
        detail: format!("journal context object: {detail}"),
    }
}

#[cfg(test)]
mod tests;
