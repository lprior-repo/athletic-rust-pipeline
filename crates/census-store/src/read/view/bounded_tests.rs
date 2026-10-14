use super::*;
use crate::Store;
use census_domain::model::RetainedConflict;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn bounded_reads_report_physical_decodes_and_preserve_the_existing_merge() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    for (id, detail) in [("one", "first"), ("one", "updated"), ("two", "second")] {
        store.append(
            Table::Schools,
            &RetainedConflict::new("fixture", id, "subject", detail),
        )?;
    }
    let snapshot = store.snapshot();
    let mut actual = Vec::new();
    let counts = snapshot.for_each_merged_bounded(
        Table::Schools,
        ReadBudget {
            rows: 3,
            bytes: 4096,
            record_bytes: 4096,
        },
        |row: RetainedConflict, _| {
            actual.push(row);
            Ok(())
        },
    )?;
    check!(eq; counts.key_visits, 3);
    check!(eq; counts.decoded_rows, 3);
    check!(eq; counts.merged_rows, 2);
    check!(eq; actual, snapshot.scan::<RetainedConflict>(Table::Schools)?);
    let exact = snapshot.for_each_merged_bounded::<RetainedConflict>(
        Table::Schools,
        ReadBudget {
            rows: 3,
            bytes: counts.encoded_bytes,
            record_bytes: 4096,
        },
        |_, _| Ok(()),
    )?;
    check!(eq; exact, counts);
    Ok(())
}

#[test]
fn bounded_reads_refuse_whole_results_instead_of_returning_partial_counts() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    store.append(
        Table::Schools,
        &RetainedConflict::new("fixture", "one", "subject", "first"),
    )?;
    store.append(
        Table::Schools,
        &RetainedConflict::new("fixture", "two", "subject", "second"),
    )?;
    let snapshot = store.snapshot();
    check!(matches!(
        snapshot.for_each_merged_bounded::<RetainedConflict>(
            Table::Schools,
            ReadBudget {
                rows: 1,
                bytes: 4096,
                record_bytes: 4096
            },
            |_, _| Ok(())
        ),
        Err(StoreError::BudgetExceeded {
            what: "capture physical rows",
            needed: 2,
            available: 1
        })
    ));
    let mut seen = 0;
    check!(matches!(
        snapshot.for_each_merged_bounded::<RetainedConflict>(
            Table::Schools,
            ReadBudget {
                rows: 2,
                bytes: 1,
                record_bytes: 4096
            },
            |_, _| {
                seen += 1;
                Ok(())
            }
        ),
        Err(StoreError::BudgetExceeded {
            what: "capture encoded bytes",
            available: 1,
            ..
        })
    ));
    check!(eq; seen, 0);
    check!(matches!(
        snapshot.for_each_merged_bounded::<RetainedConflict>(
            Table::Schools,
            ReadBudget {
                rows: 2,
                bytes: 4096,
                record_bytes: 1
            },
            |_, _| Ok(())
        ),
        Err(StoreError::RowTooLarge { max: 1, .. })
    ));
    Ok(())
}

#[test]
fn bounded_observation_reads_keep_versions_order_and_snapshot_boundary() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    for (id, detail) in [("one", "first"), ("one", "updated"), ("two", "second")] {
        store.append(
            Table::Schools,
            &RetainedConflict::new("fixture", id, "subject", detail),
        )?;
    }
    let snapshot = store.snapshot();
    store.append(
        Table::Schools,
        &RetainedConflict::new("fixture", "later", "subject", "after"),
    )?;
    let mut expected = Vec::new();
    snapshot.for_each_observation(Table::Schools, |row: RetainedConflict| {
        expected.push(row);
        Ok(())
    })?;
    let mut actual = Vec::new();
    let mut charged = Vec::new();
    let counts = snapshot.for_each_observation_bounded(
        Table::Schools,
        ReadBudget {
            rows: 3,
            bytes: 4096,
            record_bytes: 4096,
        },
        |row: RetainedConflict, bytes| {
            actual.push(row);
            charged.push(bytes);
            Ok(())
        },
    )?;
    check!(eq; actual, expected);
    check!(eq; actual.len(), 3);
    check!(eq; counts.key_visits, 3);
    check!(eq; counts.decoded_rows, 3);
    check!(eq; counts.merged_rows, 0);
    check!(eq; charged.last().copied(), Some(counts.encoded_bytes));
    check!(charged
        .windows(2)
        .all(|pair| matches!(pair, [a, b] if a < b)));
    let exact = snapshot.for_each_observation_bounded::<RetainedConflict>(
        Table::Schools,
        ReadBudget {
            rows: 3,
            bytes: counts.encoded_bytes,
            record_bytes: 4096,
        },
        |_, _| Ok(()),
    )?;
    check!(eq; exact, counts);
    let short = counts.encoded_bytes.checked_sub(1).ok_or("empty fixture")?;
    check!(
        matches!(snapshot.for_each_observation_bounded::<RetainedConflict>(Table::Schools,
        ReadBudget { rows: 3, bytes: short, record_bytes: 4096 }, |_, _| Ok(())),
        Err(StoreError::BudgetExceeded { what: "capture encoded bytes", needed, available })
            if needed == counts.encoded_bytes && available == short)
    );
    Ok(())
}

#[test]
fn bounded_observation_callback_refusal_preserves_error_without_completion() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    for id in ["one", "two", "three"] {
        store.append(
            Table::Schools,
            &RetainedConflict::new("fixture", id, "subject", id),
        )?;
    }
    let mut calls = 0;
    let result = store
        .snapshot()
        .for_each_observation_bounded::<RetainedConflict>(
            Table::Schools,
            ReadBudget {
                rows: 3,
                bytes: 4096,
                record_bytes: 4096,
            },
            |_, _| {
                calls += 1;
                if calls == 2 {
                    return Err(StoreError::Refused {
                        detail: "second fixture callback".into(),
                    });
                }
                Ok(())
            },
        );
    check!(eq; calls, 2);
    check!(
        matches!(result, Err(StoreError::Refused { detail }) if detail == "second fixture callback")
    );
    Ok(())
}
