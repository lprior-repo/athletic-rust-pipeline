use crate::public_identity::paged_generation::{
    require_inventory, AuthorityTableV2, InventoryV2, RawObjectSource, VerifiedRawGraph,
};
use crate::{Entity, ReadBudget, StoreError, StoreResult};
use census_domain::model::{CanonicalAthlete, ReviewCase, ReviewVerdictRecord, SourceObservation};
use serde::Serialize;

pub(in crate::public_identity) use super::entry_bytes;

pub(in crate::public_identity) const RAW_BUDGET: ReadBudget = ReadBudget {
    bytes: 32 * 1024 * 1024 * 1024,
    ..super::BUDGET
};

pub(in crate::public_identity) fn require_raw_inventory(inventory: InventoryV2) -> StoreResult<()> {
    require_inventory(inventory, RAW_BUDGET)
}

#[derive(Debug, Serialize)]
pub struct GraphDemandV1 {
    tables: Vec<TableDemand>,
    sections: Vec<SectionDemand>,
    status: ScanStatus,
    authority_entry_bytes: u64,
    header_reserve_bytes: usize,
    ordinary_raw_bytes_per_table: u64,
    ordinary_section_bytes: u64,
    ordinary_total_with_header_bytes: usize,
    diagnostic_raw_bytes_per_table: u64,
    diagnostic_total_entry_bytes: u64,
    raw_rows_per_table: u64,
    raw_record_bytes: usize,
}

#[derive(Debug, Serialize)]
struct TableDemand {
    table: AuthorityTableV2,
    body_verified_inventory: InventoryV2,
}

#[derive(Debug, Serialize)]
struct SectionDemand {
    table: AuthorityTableV2,
    rows: u64,
    bytes: u64,
    max_entry_bytes: usize,
    scan: SectionScan,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum SectionScan {
    Complete,
    Provisional,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum ScanStatus {
    Complete,
    Stopped {
        table: AuthorityTableV2,
        reason: String,
    },
}

pub(in crate::public_identity) fn measure<S: RawObjectSource>(
    view: &VerifiedRawGraph<'_, S>,
) -> StoreResult<GraphDemandV1> {
    let tables = AuthorityTableV2::ALL
        .into_iter()
        .map(|table| {
            Ok(TableDemand {
                table,
                body_verified_inventory: view.table_inventory(table)?,
            })
        })
        .collect::<StoreResult<Vec<_>>>()?;
    let mut report = GraphDemandV1 {
        tables,
        sections: Vec::with_capacity(4),
        status: ScanStatus::Complete,
        authority_entry_bytes: 0,
        header_reserve_bytes: super::CONTEXT_RECORD_BYTES,
        ordinary_raw_bytes_per_table: super::BUDGET.bytes,
        ordinary_section_bytes: super::BUDGET.bytes,
        ordinary_total_with_header_bytes: super::CONTEXT_TOTAL_BYTES,
        diagnostic_raw_bytes_per_table: RAW_BUDGET.bytes,
        diagnostic_total_entry_bytes: RAW_BUDGET.bytes,
        raw_rows_per_table: RAW_BUDGET.rows,
        raw_record_bytes: RAW_BUDGET.record_bytes,
    };
    for table in [
        AuthorityTableV2::Athletes,
        AuthorityTableV2::SourceObservations,
        AuthorityTableV2::ReviewCases,
        AuthorityTableV2::IdentityVerdicts,
    ] {
        if let Err(error) = report.section(view, table) {
            report.status = ScanStatus::Stopped {
                table,
                reason: error.to_string(),
            };
            break;
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests;

impl GraphDemandV1 {
    fn section<S: RawObjectSource>(
        &mut self,
        view: &VerifiedRawGraph<'_, S>,
        table: AuthorityTableV2,
    ) -> StoreResult<()> {
        require_raw_inventory(view.table_inventory(table)?)?;
        let mut section = SectionDemand {
            table,
            rows: 0,
            bytes: 0,
            max_entry_bytes: 0,
            scan: SectionScan::Provisional,
        };
        let result = match table {
            AuthorityTableV2::Athletes => merged::<CanonicalAthlete, S>(
                view,
                &mut section,
                "athlete",
                self.authority_entry_bytes,
            ),
            AuthorityTableV2::SourceObservations => view
                .for_each_observation::<SourceObservation>(table, RAW_BUDGET, |row, _| {
                    section.record("observation", &row, self.authority_entry_bytes)
                })
                .map(|_| ()),
            AuthorityTableV2::ReviewCases => {
                merged::<ReviewCase, S>(view, &mut section, "case", self.authority_entry_bytes)
            }
            AuthorityTableV2::IdentityVerdicts => merged::<ReviewVerdictRecord, S>(
                view,
                &mut section,
                "verdict",
                self.authority_entry_bytes,
            ),
            AuthorityTableV2::Performances | AuthorityTableV2::RelayResults => {
                return Err(crate::PublicIdentityError::OperationConflict.into())
            }
        };
        if result.is_ok() {
            section.scan = SectionScan::Complete;
        }
        self.authority_entry_bytes = self
            .authority_entry_bytes
            .checked_add(section.bytes)
            .ok_or(StoreError::CounterOverflow)?;
        self.sections.push(section);
        result
    }
}

fn merged<T: Entity, S: RawObjectSource>(
    view: &VerifiedRawGraph<'_, S>,
    section: &mut SectionDemand,
    kind: &'static str,
    before: u64,
) -> StoreResult<()> {
    view.for_each_merged::<T>(section.table, RAW_BUDGET, |row, _| {
        section.record(kind, &row, before)
    })?;
    Ok(())
}

impl SectionDemand {
    fn record(&mut self, kind: &'static str, row: &impl Serialize, before: u64) -> StoreResult<()> {
        let entry = entry_bytes(kind, self.rows, row)?;
        let rows = self
            .rows
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        let bytes = self
            .bytes
            .checked_add(u64::try_from(entry).map_err(|_| StoreError::CounterOverflow)?)
            .ok_or(StoreError::CounterOverflow)?;
        let total = before
            .checked_add(bytes)
            .ok_or(StoreError::CounterOverflow)?;
        if rows > RAW_BUDGET.rows {
            return Err(StoreError::BudgetExceeded {
                what: "diagnostic authority section rows",
                needed: rows,
                available: RAW_BUDGET.rows,
            });
        }
        if total > RAW_BUDGET.bytes {
            return Err(StoreError::BudgetExceeded {
                what: "diagnostic authority entry bytes",
                needed: total,
                available: RAW_BUDGET.bytes,
            });
        }
        self.rows = rows;
        self.bytes = bytes;
        if entry > self.max_entry_bytes {
            self.max_entry_bytes = entry;
        }
        Ok(())
    }
}
