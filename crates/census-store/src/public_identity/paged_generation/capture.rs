mod binding;
mod scan;
pub(in crate::public_identity) mod verify;

use super::{
    admission::Node, CaptureBindingV2, DigestV2, ObjectRefV2, RawGenerationOperationIdV2,
    RawGenerationSealV2,
};
use crate::public_identity::local_adjudication::admission::graph::{
    load_graph, reserve_graph, GraphPlanBuilder, GraphReservation,
};
use crate::public_identity::PublicIdentityWriterV1;
use crate::{Store, StoreError, StoreResult, StoreSnapshot};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedRawGenerationV2 {
    binding: CaptureBindingV2,
    seal: ObjectRefV2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawGenerationCaptureOutcomeV2 {
    Captured(RetainedRawGenerationV2),
    HistoricalReplay(RetainedRawGenerationV2),
}

impl RawGenerationCaptureOutcomeV2 {
    pub fn retained(&self) -> &RetainedRawGenerationV2 {
        match self {
            Self::Captured(retained) | Self::HistoricalReplay(retained) => retained,
        }
    }
}

impl RetainedRawGenerationV2 {
    pub fn binding(&self) -> &CaptureBindingV2 {
        &self.binding
    }
    pub fn seal(&self) -> &ObjectRefV2 {
        &self.seal
    }
    pub fn verify(&self, snapshot: &StoreSnapshot<'_>) -> StoreResult<RawGenerationSealV2> {
        verify::verify(snapshot, &self.binding, &self.seal, |_, _| Ok(()))
    }
}

impl PublicIdentityWriterV1 {
    pub fn capture_raw_generation_v2(
        &self,
        store: &Store,
        operation: &RawGenerationOperationIdV2,
    ) -> StoreResult<RawGenerationCaptureOutcomeV2> {
        self.require_current_qualification()?;
        let guard = store.lock_appends();
        self.verified(store)?;
        let held = store.snapshot_locked();
        let current = binding::capture(store, &held)?;
        let reserved = load_graph(store, self.store_identity(), operation)?;
        if let Some(reserved) = &reserved {
            if reserved.is_complete(store)? {
                return completed(&held, reserved)
                    .map(RawGenerationCaptureOutcomeV2::HistoricalReplay);
            }
            binding::require_inputs(reserved.capture(), &current)?;
        }
        let capture = match &reserved {
            Some(reserved) => reserved.capture().clone(),
            None => current,
        };
        let mut builder = GraphPlanBuilder::new(capture.clone());
        let seal = scan::walk(&held, &capture, |node| builder.add(store, node))?;
        let plan = builder.finish(store, &seal)?;
        let reserved = match reserved {
            Some(reserved) => {
                reserved.require_plan(&plan)?;
                reserved
            }
            None => reserve_graph(store, operation, plan)?,
        };
        drop(guard);
        stage(store, &held, &reserved)?;
        finish_current(store, &reserved).map(RawGenerationCaptureOutcomeV2::Captured)
    }
}

fn stage(store: &Store, held: &StoreSnapshot<'_>, reserved: &GraphReservation) -> StoreResult<()> {
    let seal = scan::walk(held, reserved.capture(), |node| {
        stage_node(store, reserved, node)
    })?;
    {
        let _guard = store.lock_appends();
        binding::require_inputs(
            reserved.capture(),
            &binding::capture(store, &store.snapshot_locked())?,
        )?;
    }
    let object = stage_node(store, reserved, Node::Seal(&seal))?;
    if &object != reserved.seal() {
        return Err(super::invalid("staged capture seal changed"));
    }
    Ok(())
}

fn stage_node(
    store: &Store,
    reserved: &GraphReservation,
    node: Node<'_>,
) -> StoreResult<ObjectRefV2> {
    let measured = node.measure(store)?;
    let object = ObjectRefV2::new(
        DigestV2::parse(measured.root())?,
        u32::try_from(measured.jsonl_bytes()).map_err(|_| StoreError::CounterOverflow)?,
        u32::try_from(measured.records()).map_err(|_| StoreError::CounterOverflow)?,
        measured.full_bytes(),
    )?;
    let permit = reserved.permit_for(store, object.root().as_str())?;
    node.stage(store, &permit)?;
    Ok(object)
}

fn finish_current(
    store: &Store,
    reserved: &GraphReservation,
) -> StoreResult<RetainedRawGenerationV2> {
    let _guard = store.lock_appends();
    let retained = store.snapshot_locked();
    binding::require_inputs(reserved.capture(), &binding::capture(store, &retained)?)?;
    completed(&retained, reserved)
}

fn completed(
    snapshot: &StoreSnapshot<'_>,
    reserved: &GraphReservation,
) -> StoreResult<RetainedRawGenerationV2> {
    verify::verify(snapshot, reserved.capture(), reserved.seal(), |_, _| Ok(()))?;
    Ok(RetainedRawGenerationV2 {
        binding: reserved.capture().clone(),
        seal: reserved.seal().clone(),
    })
}

#[cfg(test)]
mod scan_tests;
#[cfg(test)]
mod tests;
