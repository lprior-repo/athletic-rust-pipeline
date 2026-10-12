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
