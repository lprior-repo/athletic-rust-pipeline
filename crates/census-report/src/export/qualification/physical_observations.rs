use crate::export::{proof_objects, ExportDataset};
use crate::report::ReportResult;
use census_store::{Entity, ReadBudget, ReadCounts, StoreError, StoreResult, StoreSnapshot, Table};
use serde::{Deserialize, Serialize};
use std::path::Path;

mod capture;
mod closure;
mod descriptor;
mod object;
mod reader;
mod writer;

#[cfg(test)]
mod tests;

use closure::Closure;
pub(crate) use descriptor::PhysicalObservationDescriptor;
use object::ObjectRef;
pub(super) use writer::Writer;

const CHUNK_BYTES: usize = 4 * 1024 * 1024;
const RECORD_BYTES: usize = 1024 * 1024;
const MAX_CHUNKS: usize = 8192;
const INVENTORY_BYTES: u64 = 4 * 1024 * 1024;
const MAX_BYTES: u64 = 16 * 1024 * 1024 * 1024;

pub(crate) const OBSERVATION_BUDGET: ReadBudget = ReadBudget {
    rows: 4_000_000,
    bytes: MAX_BYTES,
    record_bytes: RECORD_BYTES,
};

pub(crate) fn capture<T: Entity>(
    dataset: &ExportDataset,
    snapshot: &StoreSnapshot<'_>,
    table: Table,
    objects_dir: &Path,
    budget: ReadBudget,
) -> ReportResult<PhysicalObservationDescriptor> {
    capture::capture::<T>(dataset, snapshot, table, objects_dir, budget)
}

pub(crate) fn visit<T: Entity>(
    dataset: &ExportDataset,
    objects_dir: &Path,
    descriptor: &PhysicalObservationDescriptor,
    expected_fingerprint: &str,
    budget: ReadBudget,
    visit: impl FnMut(T) -> StoreResult<()>,
) -> ReportResult<ReadCounts> {
    dataset.ensure_input_binding()?;
    descriptor.validate(dataset, expected_fingerprint, budget)?;
    reader::visit(objects_dir, descriptor, budget, visit)
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Inventory {
    version: u32,
    table: Table,
    chunks: Vec<ObjectRef>,
    rows: u64,
    bytes: u64,
    digest: String,
}

fn invalid(detail: &str) -> StoreError {
    StoreError::Refused {
        detail: detail.to_string(),
    }
}

fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn admission(table: Table, budget: ReadBudget) -> StoreResult<()> {
    if !matches!(
        table,
        Table::Athletes | Table::Performances | Table::Teams | Table::Meets | Table::Events
    ) {
        return Err(invalid(
            "physical proof inventory requires a supported observation table",
        ));
    }
    if budget.rows > census_store::MAX_ROWS_PER_TABLE
        || budget.bytes > MAX_BYTES
        || budget.record_bytes > RECORD_BYTES
    {
        return Err(invalid(
            "physical proof inventory budget exceeds its hard ceiling",
        ));
    }
    Ok(())
}
