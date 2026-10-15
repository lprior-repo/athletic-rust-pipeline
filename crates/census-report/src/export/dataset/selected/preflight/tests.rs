use super::{admit, admit_with_budget, PROOF_BUDGET};
use crate::report::ReportError;
use census_domain::model::{CanonicalMeet, CompetitionLevel};
use census_store::{ReadBudget, Store, StoreError, Table};
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn selected_preflight_preserves_existing_physical_proof_limits() -> TestResult {
    check!(eq; PROOF_BUDGET.rows, 200_000);
    check!(eq; PROOF_BUDGET.bytes, 256 * 1024 * 1024);
    check!(eq; PROOF_BUDGET.record_bytes, 1024 * 1024);
    Ok(())
}

#[test]
fn selected_preflight_checks_all_five_tables_before_oversized_row_decode() -> TestResult {
    let oversized = json!({"id": "not-a-domain-row", "padding": "x".repeat(1024 * 1024)});
    let bytes = serde_json::to_vec(&oversized)?.len();
    for table in [
        Table::Athletes,
        Table::Performances,
        Table::Teams,
        Table::Meets,
        Table::Events,
    ] {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path())?;
        store.append(table, &oversized)?;
        check!(matches!(admit(&store.snapshot()),
            Err(ReportError::Store(StoreError::RowTooLarge { bytes: actual, max }))
                if actual == bytes && max == PROOF_BUDGET.record_bytes));
    }
    Ok(())
}

#[test]
fn selected_preflight_accepts_the_existing_one_mib_record_boundary() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let mut row = meet()?;
    row.location = Some(String::new());
    let overhead = serde_json::to_vec(&row)?.len();
    let padding = PROOF_BUDGET
        .record_bytes
        .checked_sub(overhead)
        .ok_or("record overhead")?;
    row.location = Some("x".repeat(padding));
    check!(eq; serde_json::to_vec(&row)?.len(), PROOF_BUDGET.record_bytes);
    store.append(Table::Meets, &row)?;
    admit(&store.snapshot())?;
    check!(eq; store.snapshot().scan::<CanonicalMeet>(Table::Meets)?, [row]);
    Ok(())
}

#[test]
fn selected_preflight_charges_duplicate_physical_rows_before_decoding_excess() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let row = meet()?;
    store.append_many(Table::Meets, &[row.clone(), row.clone()])?;
    store.append(Table::Meets, &json!({"id": row.id}))?;
    let budget = ReadBudget {
        rows: 2,
        ..PROOF_BUDGET
    };
    check!(matches!(
        admit_with_budget(&store.snapshot(), budget),
        Err(ReportError::Store(StoreError::BudgetExceeded {
            what: "capture physical rows",
            needed: 3,
            available: 2,
        }))
    ));
    Ok(())
}

#[test]
fn selected_preflight_charges_encoded_bytes_before_invalid_row_decode() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    store.append(Table::Events, &json!({"id": "invalid-event"}))?;
    let budget = ReadBudget {
        bytes: 1,
        ..PROOF_BUDGET
    };
    check!(matches!(admit_with_budget(&store.snapshot(), budget),
        Err(ReportError::Store(StoreError::BudgetExceeded {
            what: "capture encoded bytes", needed, available: 1,
        })) if needed > 1));
    Ok(())
}

#[test]
fn selected_preflight_exact_bytes_and_rows_preserve_merged_semantics() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let row = meet()?;
    let record_bytes = serde_json::to_vec(&row)?.len();
    store.append_many(Table::Meets, &[row.clone(), row])?;
    let snapshot = store.snapshot();
    let before = snapshot.scan::<CanonicalMeet>(Table::Meets)?;
    let fingerprint = snapshot.tables_digest(&[Table::Meets])?;
    let counts = snapshot.for_each_observation_bounded::<CanonicalMeet>(
        Table::Meets,
        PROOF_BUDGET,
        |_, _| Ok(()),
    )?;
    let exact = ReadBudget {
        rows: counts.key_visits,
        bytes: counts.encoded_bytes,
        record_bytes,
    };
    admit_with_budget(&snapshot, exact)?;
    check!(eq; before.len(), 1);
    check!(eq; counts.decoded_rows, 2);
    check!(eq; snapshot.scan::<CanonicalMeet>(Table::Meets)?, before);
    check!(eq; snapshot.tables_digest(&[Table::Meets])?, fingerprint);
    let short = exact.bytes.checked_sub(1).ok_or("byte lower bound")?;
    check!(
        matches!(admit_with_budget(&snapshot, ReadBudget { bytes: short, ..exact }),
        Err(ReportError::Store(StoreError::BudgetExceeded {
            what: "capture encoded bytes", needed, available,
        })) if needed == exact.bytes && available == short)
    );
    Ok(())
}

#[test]
fn selected_preflight_leaves_other_input_tables_unqualified() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    store.append(
        Table::Schools,
        &json!({"id": "unqualified", "padding": "x".repeat(1024 * 1024)}),
    )?;
    admit(&store.snapshot())?;
    Ok(())
}

fn meet() -> Result<CanonicalMeet, String> {
    CanonicalMeet::new_checked(
        None,
        "Selected preflight",
        "2026-09-01",
        CompetitionLevel::Dual,
    )
}
