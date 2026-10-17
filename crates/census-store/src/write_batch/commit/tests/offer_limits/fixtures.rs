use super::*;
use crate::{ConditionalBatch, ObservationBudget, StoreBatch};
use census_domain::model::normalize_name;
use census_domain::UsJurisdiction;
use std::path::Path;

const PADDED_NAME_BYTES: usize = 225_000;

fn school(label: &str) -> TestResult<CanonicalSchool> {
    let name = format!("Offer Limits {label}");
    let normalized = normalize_name(&name);
    let (row, _) = CanonicalSchool::new(UsJurisdiction::Wisconsin, name, normalized, None);
    Ok(row)
}

pub(super) fn schools(prefix: &str, count: usize) -> TestResult<Vec<CanonicalSchool>> {
    let rows = (0..count)
        .map(|index| school(&format!("{prefix}-{index}")))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub(super) fn padded_school(index: usize) -> TestResult<CanonicalSchool> {
    let name = format!("{} {index}", "x".repeat(PADDED_NAME_BYTES));
    let normalized = normalize_name(&name);
    let (row, _) = CanonicalSchool::new(UsJurisdiction::Wisconsin, name, normalized, None);
    Ok(row)
}

pub(super) fn staged_batch<'s>(
    store: &'s Store,
    rows: &[CanonicalSchool],
) -> TestResult<StoreBatch<'s>> {
    let mut batch = store.write_batch();
    batch.append_many(Table::Schools, rows)?;
    Ok(batch)
}

pub(super) fn child<'s>(
    store: &'s Store,
    operation: &'s str,
    digest: &'s str,
    rows: &[CanonicalSchool],
) -> TestResult<ConditionalBatch<'s, 's>> {
    let batch = staged_batch(store, rows)?;
    let effect = batch.conditional(operation, digest)?;
    Ok(effect)
}

pub(super) fn offered_bytes(rows: &[CanonicalSchool]) -> TestResult<usize> {
    rows.iter()
        .try_fold(0usize, |total, row| -> TestResult<usize> {
            let bytes = total
                .checked_add(ObservationBudget::record_bytes(row)?)
                .ok_or("offered byte sum overflow")?;
            Ok(bytes)
        })
}

pub(super) fn smallest_offered_row(
    outer: &[CanonicalSchool],
    first: &[CanonicalSchool],
    second: &[CanonicalSchool],
) -> TestResult<usize> {
    outer
        .iter()
        .chain(first.iter())
        .chain(second.iter())
        .try_fold(usize::MAX, |smallest, row| -> TestResult<usize> {
            Ok(smallest.min(ObservationBudget::record_bytes(row)?))
        })
}

pub(super) fn receipt_bytes(store: &Store, operation: &str) -> TestResult<Vec<u8>> {
    let held = store
        .receipts
        .get(operation.as_bytes())?
        .ok_or("receipt bytes are missing")?;
    Ok(held.to_vec())
}

pub(super) fn assert_retained(
    store: &Store,
    sequence: u64,
    generation: u64,
    rows: usize,
) -> TestResult {
    let counted = u64::try_from(rows)?;
    check!(eq; store.snapshot().sequence(), sequence);
    check!(eq; store.evidence_generation(), generation);
    check!(eq; store.count(Table::Schools)?, counted);
    check!(eq; store.walk_table(Table::Schools)?.rows, counted);
    check!(eq; store.scan::<CanonicalSchool>(Table::Schools)?.len(), rows);
    Ok(())
}

pub(super) fn assert_reopened_without_writes(root: &Path, operations: &[&str]) -> TestResult {
    let store = Store::open(root)?;
    check!(eq; store.count(Table::Schools)?, 0);
    check!(eq; store.walk_table(Table::Schools)?.rows, 0);
    check!(eq; store.scan::<CanonicalSchool>(Table::Schools)?.len(), 0);
    for operation in operations {
        check!(
            store.receipt(operation)?.is_none(),
            "receipt for {operation}"
        );
    }
    check!(eq; store.receipt_count()?, 0);
    Ok(())
}
