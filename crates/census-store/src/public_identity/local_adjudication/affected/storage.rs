use super::{Inputs, Physical};
use crate::journal::context::{measure_context_object, ContextRecordSink, StagedContextObject};
use crate::public_identity::context::{storage::charge_section_bytes, wire};
use crate::{Store, StoreError, StoreResult};
use serde::Serialize;

impl Inputs {
    pub(in crate::public_identity) fn admission_charge(
        &self,
        store: &Store,
    ) -> StoreResult<crate::journal::context::ContextCharge> {
        crate::journal::context::charge_context_object(store, |sink| self.emit_context(sink))
    }

    pub(in crate::public_identity) fn stage_admitted(
        &self,
        store: &Store,
        permit: &crate::public_identity::local_adjudication::admission::Permit<'_>,
    ) -> StoreResult<StagedContextObject> {
        crate::journal::context::stage_reserved_context(store, permit, |sink| {
            self.emit_context(sink)
        })
    }

    pub(in crate::public_identity) fn restore_admission_binding(
        &mut self,
        binding: &super::super::admission::Binding<'_>,
    ) -> StoreResult<()> {
        if &self.header.store != binding.store
            || self.header.authority_context != binding.authority_context
            || self.header.evidence_generation != binding.evidence_generation
            || self.header.derived_generation != binding.derived_generation
        {
            return Err(super::invalid("manual admission evidence moved"));
        }
        self.header.snapshot_sequence = binding.snapshot_sequence;
        Ok(())
    }

    pub(in crate::public_identity) fn measure_context(&self) -> StoreResult<String> {
        measure_context_object(|sink| self.emit_context(sink))
    }

    fn emit_context(&self, sink: &mut ContextRecordSink<'_>) -> StoreResult<()> {
        emit(sink, "header", "header", &self.header)?;
        emit_rows(sink, "performance", &self.performances)?;
        emit_rows(sink, "relay", &self.relays)
    }
}

fn emit_rows(sink: &mut ContextRecordSink<'_>, kind: &str, rows: &[Physical]) -> StoreResult<()> {
    rows.iter()
        .enumerate()
        .try_fold(0u64, |bytes, (index, row)| {
            let before = sink.written_bytes();
            emit(sink, &format!("{kind}:{index:020}"), kind, row)?;
            let added = sink
                .written_bytes()
                .checked_sub(before)
                .ok_or(StoreError::CounterOverflow)?;
            charge_section_bytes(bytes, added)
        })?;
    Ok(())
}

fn emit<T: Serialize>(
    sink: &mut ContextRecordSink<'_>,
    key: &str,
    kind: &str,
    value: &T,
) -> StoreResult<()> {
    sink.record(&wire::Entry {
        key,
        at: "",
        payload: wire::Payload { kind, value },
    })
}
