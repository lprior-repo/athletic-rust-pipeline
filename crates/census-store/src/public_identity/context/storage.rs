use super::{wire, Inputs, BUDGET};
use crate::journal::context::{
    measure_context_object, stage_context_object, ContextRecordSink, StagedContextObject,
};
use crate::{Store, StoreError, StoreResult};
use census_domain::model::StoreIdentity;
use serde::Serialize;

pub(super) const RETAINED_SECTION_BYTES: u64 = 16 * 1024 * 1024;

pub(super) fn charge_section_bytes(current: u64, added: usize) -> StoreResult<u64> {
    let added = u64::try_from(added).map_err(|_| StoreError::CounterOverflow)?;
    let needed = current
        .checked_add(added)
        .ok_or(StoreError::CounterOverflow)?;
    if needed > RETAINED_SECTION_BYTES {
        return Err(StoreError::BudgetExceeded {
            what: "retained authority section bytes",
            needed,
            available: RETAINED_SECTION_BYTES,
        });
    }
    Ok(needed)
}

impl Inputs {
    pub(in crate::public_identity) fn stage(
        &self,
        store: &Store,
        lineage: &StoreIdentity,
    ) -> StoreResult<StagedContextObject> {
        stage_context_object(store, |sink| self.emit_context(sink, lineage))
    }

    pub(in crate::public_identity) fn measure_context(
        &self,
        lineage: &StoreIdentity,
    ) -> StoreResult<String> {
        measure_context_object(|sink| self.emit_context(sink, lineage))
    }

    fn emit_context(
        &self,
        sink: &mut ContextRecordSink<'_>,
        lineage: &StoreIdentity,
    ) -> StoreResult<()> {
        let header = wire::Header {
            version: 1,
            authority_policy: 1,
            enrichment_policy: 1,
            lineage: lineage.clone(),
            counts: [
                self.athletes.len(),
                self.observations.len(),
                self.cases.len(),
                self.verdicts.len(),
            ],
        };
        emit(sink, "header", "header", &header)?;
        emit_rows(sink, "athlete", &self.athletes)?;
        emit_rows(sink, "observation", &self.observations)?;
        emit_rows(sink, "case", &self.cases)?;
        emit_rows(sink, "verdict", &self.verdicts)
    }
}

fn emit_rows<T: Serialize>(
    sink: &mut ContextRecordSink<'_>,
    kind: &str,
    rows: &[T],
) -> StoreResult<()> {
    let needed = u64::try_from(rows.len()).map_err(|_| StoreError::CounterOverflow)?;
    if needed > BUDGET.rows {
        return Err(StoreError::BudgetExceeded {
            what: "authority context rows",
            needed,
            available: BUDGET.rows,
        });
    }
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
