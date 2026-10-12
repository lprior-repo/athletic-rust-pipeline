use crate::{ObservationBudget, ObservationChunk, Store, StoreError, Table};
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use std::cell::Cell;

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct Row<'a> {
    payload: &'a str,
    materialized: &'a Cell<usize>,
}

impl Serialize for Row<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if std::any::type_name::<S>().contains("alloc::vec::Vec")
            || std::any::type_name::<S>().contains("budget::Buffer")
        {
            self.materialized
                .set(self.materialized.get().saturating_add(1));
        }
        let mut row = serializer.serialize_struct("Row", 2)?;
        row.serialize_field("id", "budget-row")?;
        row.serialize_field("payload", self.payload)?;
        row.end()
    }
}

#[test]
fn append_many_refuses_record_byte_overrun_before_materialization() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let payload = "x".repeat(1024 * 1024);
    let materialized = Cell::new(0);
    let mut batch = store.write_batch();
    let result = batch.append_many(
        Table::Schools,
        &[Row {
            payload: &payload,
            materialized: &materialized,
        }],
    );
    check!(
        matches!(result, Err(StoreError::RowTooLarge { max: 1_048_576, .. })),
        "{result:?}"
    );
    check!(eq; materialized.get(), 0);
    check!(batch.is_empty());
    Ok(())
}

#[test]
fn append_many_refuses_batch_byte_overrun_before_materialization() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let payload = "x".repeat(512 * 1024);
    let materialized = Cell::new(0);
    let rows: Vec<_> = (0..16)
        .map(|_| Row {
            payload: &payload,
            materialized: &materialized,
        })
        .collect();
    let mut batch = store.write_batch();
    let result = batch.append_many(Table::Schools, &rows);
    check!(
        matches!(
            result,
            Err(StoreError::BudgetExceeded {
                what: "observation batch bytes",
                available: 8_388_608,
                ..
            })
        ),
        "{result:?}"
    );
    check!(eq; materialized.get(), 0);
    check!(batch.is_empty());
    Ok(())
}

#[test]
fn an_admitted_record_materializes_and_commits() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let materialized = Cell::new(0);
    let mut batch = store.write_batch();
    batch.append_many(
        Table::Schools,
        &[Row {
            payload: "small",
            materialized: &materialized,
        }],
    )?;
    check!(eq; materialized.get(), 1);
    batch.commit()?;
    check!(eq; store.count(Table::Schools)?, 1);
    Ok(())
}

#[test]
fn aggregate_budget_includes_rows_staged_by_previous_calls() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let payload = "x".repeat(512 * 1024);
    let materialized = Cell::new(0);
    let rows: Vec<_> = (0..15)
        .map(|_| Row {
            payload: &payload,
            materialized: &materialized,
        })
        .collect();
    let mut batch = store.write_batch();
    batch.append_many(Table::Schools, &rows)?;
    materialized.set(0);
    let result = batch.append_many(Table::Meets, &rows[..1]);
    check!(
        matches!(
            result,
            Err(StoreError::BudgetExceeded {
                what: "observation batch bytes",
                available: 8_388_608,
                ..
            })
        ),
        "{result:?}"
    );
    check!(eq; materialized.get(), 0);
    batch.commit()?;
    check!(eq; store.count(Table::Schools)?, 15);
    check!(eq; store.count(Table::Meets)?, 0);
    Ok(())
}

#[test]
fn byte_chunks_are_deterministic_and_bind_every_row_exactly_once() -> TestResult {
    let payload = "x".repeat(512 * 1024);
    let rows = vec![payload; 16];
    let expected = vec![
        ObservationChunk {
            rows: 0..15,
            encoded_bytes: 7_864_350,
        },
        ObservationChunk {
            rows: 15..16,
            encoded_bytes: 524_290,
        },
    ];
    let manifest = ObservationBudget::chunks(&rows)?;
    check!(eq; manifest, expected);
    check!(eq; ObservationBudget::chunks(&rows)?, manifest);
    for chunk in manifest {
        let rows = rows.get(chunk.rows).ok_or("manifest range invalid")?;
        let measured = rows.iter().try_fold(
            0usize,
            |bytes, row| -> Result<usize, Box<dyn std::error::Error>> {
                bytes
                    .checked_add(serde_json::to_vec(row)?.len())
                    .ok_or_else(|| "test size overflow".into())
            },
        )?;
        check!(eq; measured, chunk.encoded_bytes);
        check!(measured <= crate::MAX_OBSERVATION_BATCH_BYTES);
        check!(rows.len() <= crate::MAX_OBSERVATION_BATCH_ROWS);
    }
    Ok(())
}

#[test]
fn record_boundaries_count_json_escaping_and_utf8() -> TestResult {
    check!(eq;
        ObservationBudget::record_bytes(&"\\n🦀")?,
        serde_json::to_vec(&"\\n🦀")?.len()
    );
    let row = "x".repeat(crate::MAX_OBSERVATION_RECORD_BYTES - 2);
    check!(eq;
        ObservationBudget::record_bytes(&row)?,
        crate::MAX_OBSERVATION_RECORD_BYTES
    );
    let oversized = format!("{row}x");
    check!(matches!(
        ObservationBudget::record_bytes(&oversized),
        Err(StoreError::RowTooLarge {
            bytes: 1_048_577,
            max: 1_048_576
        })
    ));
    check!(eq; ObservationBudget::chunks(&Vec::<String>::new())?, vec![]);
    check!(eq;
        ObservationBudget::admit(std::iter::empty::<&String>(), 0, 0)?,
        (0, 0)
    );
    Ok(())
}

#[test]
fn row_work_is_refused_before_any_row_is_serialized() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let materialized = Cell::new(0);
    let rows: Vec<_> = (0..4097)
        .map(|_| Row {
            payload: "",
            materialized: &materialized,
        })
        .collect();
    let mut batch = store.write_batch();
    let result = batch.append_many(Table::Schools, &rows);
    check!(matches!(
        result,
        Err(StoreError::BudgetExceeded {
            what: "observation batch rows",
            needed: 4097,
            available: 4096
        })
    ));
    check!(eq; materialized.get(), 0);
    check!(batch.is_empty());
    Ok(())
}
