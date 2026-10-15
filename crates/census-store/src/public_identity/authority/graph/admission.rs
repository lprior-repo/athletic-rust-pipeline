use super::{AuthorityTableV2, RawObjectSource, VerifiedRawGraph, BUDGET};
use crate::journal::context::{
    measure_context_object, ContextRecordSink, CONTEXT_RECORD_BYTES, CONTEXT_TOTAL_BYTES,
};
use crate::public_identity::context::{storage::charge_section_bytes, wire};
use crate::{Entity, StoreError, StoreResult};
use census_domain::model::{CanonicalAthlete, ReviewCase, ReviewVerdictRecord, SourceObservation};
use serde::Serialize;

pub(super) fn preflight<S: RawObjectSource>(view: &VerifiedRawGraph<'_, S>) -> StoreResult<()> {
    measure_context_object(|sink| {
        merged::<CanonicalAthlete, S>(view, sink, AuthorityTableV2::Athletes, "athlete")?;
        let mut section = Section::new("observation");
        view.for_each_observation::<SourceObservation>(
            AuthorityTableV2::SourceObservations,
            BUDGET,
            |row, _| section.record(sink, &row),
        )?;
        merged::<ReviewCase, S>(view, sink, AuthorityTableV2::ReviewCases, "case")?;
        merged::<ReviewVerdictRecord, S>(view, sink, AuthorityTableV2::IdentityVerdicts, "verdict")
    })?;
    Ok(())
}

fn merged<T: Entity, S: RawObjectSource>(
    view: &VerifiedRawGraph<'_, S>,
    sink: &mut ContextRecordSink<'_>,
    table: AuthorityTableV2,
    kind: &'static str,
) -> StoreResult<()> {
    let mut section = Section::new(kind);
    view.for_each_merged::<T>(table, BUDGET, |row, _| section.record(sink, &row))?;
    Ok(())
}

struct Section {
    kind: &'static str,
    rows: u64,
    bytes: u64,
}

impl Section {
    fn new(kind: &'static str) -> Self {
        Self {
            kind,
            rows: 0,
            bytes: 0,
        }
    }

    fn record<T: Serialize>(
        &mut self,
        sink: &mut ContextRecordSink<'_>,
        row: &T,
    ) -> StoreResult<()> {
        let rows = self
            .rows
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        if rows > BUDGET.rows {
            return Err(StoreError::BudgetExceeded {
                what: "authority context rows",
                needed: rows,
                available: BUDGET.rows,
            });
        }
        let before = sink.written_bytes();
        sink.record(&wire::Entry {
            key: &format!("{}:{:020}", self.kind, self.rows),
            at: "",
            payload: wire::Payload {
                kind: self.kind,
                value: row,
            },
        })?;
        let added = sink
            .written_bytes()
            .checked_sub(before)
            .ok_or(StoreError::CounterOverflow)?;
        self.bytes = charge_section_bytes(self.bytes, added)?;
        aggregate(sink.written_bytes())?;
        self.rows = rows;
        Ok(())
    }
}

fn aggregate(sections: usize) -> StoreResult<()> {
    let needed = sections
        .checked_add(CONTEXT_RECORD_BYTES)
        .ok_or(StoreError::CounterOverflow)?;
    if needed > CONTEXT_TOTAL_BYTES {
        return Err(StoreError::BudgetExceeded {
            what: "graph authority sections with header reserve",
            needed: u64::try_from(needed).map_err(|_| StoreError::CounterOverflow)?,
            available: u64::try_from(CONTEXT_TOTAL_BYTES)
                .map_err(|_| StoreError::CounterOverflow)?,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;

pub(super) fn affected<S: RawObjectSource>(view: &VerifiedRawGraph<'_, S>) -> StoreResult<()> {
    measure_context_object(|sink| {
        physical::<census_domain::model::CanonicalPerformance, S>(
            view,
            sink,
            AuthorityTableV2::Performances,
            "performance",
        )?;
        physical::<census_domain::model::RelayResult, S>(
            view,
            sink,
            AuthorityTableV2::RelayResults,
            "relay",
        )
    })?;
    Ok(())
}

fn physical<T: Entity, S: RawObjectSource>(
    view: &VerifiedRawGraph<'_, S>,
    sink: &mut ContextRecordSink<'_>,
    table: AuthorityTableV2,
    kind: &'static str,
) -> StoreResult<()> {
    let mut section = Section::new(kind);
    view.for_each_physical::<T>(table, BUDGET, |_, key, raw, _, _| {
        let row =
            crate::public_identity::local_adjudication::affected::Physical::from_raw(key, raw)?;
        section.record(sink, &row)
    })?;
    Ok(())
}
