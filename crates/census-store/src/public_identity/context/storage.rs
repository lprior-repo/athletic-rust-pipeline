#[cfg(test)]
use super::{wire, Inputs, BUDGET};
#[cfg(test)]
use crate::journal::context::{
    measure_context_object, stage_context_object, ContextRecordSink, StagedContextObject,
};
#[cfg(test)]
use crate::Store;
use crate::{StoreError, StoreResult};
#[cfg(test)]
use census_domain::model::StoreIdentity;
#[cfg(test)]
use serde::Serialize;

pub(super) const RETAINED_SECTION_BYTES: u64 = 16 * 1024 * 1024;

pub(in crate::public_identity) fn charge_section_bytes(
    current: u64,
    added: usize,
) -> StoreResult<u64> {
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

#[cfg(test)]
impl Inputs {
    pub(in crate::public_identity) fn admission_charge(
        &self,
        store: &Store,
        lineage: &StoreIdentity,
    ) -> StoreResult<crate::journal::context::ContextCharge> {
        crate::journal::context::charge_context_object(store, |sink| {
            self.emit_context(sink, lineage)
        })
    }

    pub(in crate::public_identity) fn stage_admitted(
        &self,
        store: &Store,
        lineage: &StoreIdentity,
        permit: &crate::public_identity::local_adjudication::admission::Permit<'_>,
    ) -> StoreResult<StagedContextObject> {
        crate::journal::context::stage_reserved_context(store, permit, |sink| {
            self.emit_context(sink, lineage)
        })
    }

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

#[cfg(test)]
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

#[cfg(test)]
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
