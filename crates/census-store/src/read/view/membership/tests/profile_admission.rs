use super::*;
use crate::acquisition::qualification::public_profile;

#[test]
fn profile_inventory_admission_is_bounded_and_exclusive_to_its_phase() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let view = store.snapshot();
    let namespace = JournalNamespace::whole_phase(public_profile::PHASE)?;
    let budget = public_profile::scan_budget()?;
    check!(eq; budget.rows, 1_428_000_000);
    check!(eq; budget.bytes, 17_592_186_044_416);
    check!(eq; budget.record_bytes, MAX_JOURNAL_VALUE_BYTES);
    let inventory = view.for_each_journal_root_bounded(namespace, budget, |_| Ok(()))?;
    check!(eq; inventory.logical_roots(), 0);
    for phase in [PHASE, "public_resource_child_profile_capture_v2"] {
        let refused = view.for_each_journal_root_bounded(
            JournalNamespace::whole_phase(phase)?,
            budget,
            |_| Ok(()),
        );
        check!(matches!(
            refused,
            Err(StoreError::BudgetExceeded {
                what: "journal admitted rows",
                available: 25_000,
                ..
            })
        ));
    }
    let rows = budget
        .rows
        .checked_add(1)
        .ok_or(StoreError::CounterOverflow)?;
    let refused =
        view.for_each_journal_root_bounded(namespace, ReadBudget { rows, ..budget }, |_| Ok(()));
    check!(matches!(refused, Err(StoreError::BudgetExceeded {
        what: "journal admitted rows", needed, available,
    }) if needed == rows && available == budget.rows));
    let bytes = budget
        .bytes
        .checked_add(1)
        .ok_or(StoreError::CounterOverflow)?;
    let refused =
        view.for_each_journal_root_bounded(namespace, ReadBudget { bytes, ..budget }, |_| Ok(()));
    check!(matches!(refused, Err(StoreError::BudgetExceeded {
        what: "journal admitted bytes", needed, available,
    }) if needed == bytes && available == budget.bytes));
    Ok(())
}
