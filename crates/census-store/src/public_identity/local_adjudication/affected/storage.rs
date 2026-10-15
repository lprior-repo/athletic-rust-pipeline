use super::{Inputs, Physical};
use crate::journal::context::{
    measure_context_object, stage_context_object, ContextRecordSink, StagedContextObject,
};
use crate::public_identity::context::{storage::charge_section_bytes, wire};
use crate::{Store, StoreError, StoreResult};
use serde::Serialize;

impl Inputs {
    pub(in crate::public_identity) fn measure_context(&self) -> StoreResult<String> {
        measure_context_object(|sink| self.emit_context(sink))
    }

    pub(in crate::public_identity) fn stage(
        &self,
        store: &Store,
    ) -> StoreResult<StagedContextObject> {
        stage_context_object(store, |sink| self.emit_context(sink))
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
