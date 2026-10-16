use crate::{ReadBudget, Store, StoreError};
use std::cell::Cell;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const PREFIX: &[u8] = b"evidence_subject_attachment_v1:";
const BUDGET: ReadBudget = ReadBudget {
    rows: 2,
    bytes: 4096,
    record_bytes: 1024,
};

#[test]
fn attachment_receipt_prefix_is_pinned_and_complete() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let first = [PREFIX, b"a"].concat();
    let second = [PREFIX, b"b"].concat();
    store.receipts.insert(&first, b"first")?;
    store.receipts.insert(b"unrelated", b"other")?;
    let held = store.snapshot();
    store.receipts.insert(&second, b"second")?;
    let mut rows = Vec::new();
    let counts = held.visit_attachment_receipts_bounded(PREFIX, BUDGET, |key, raw| {
        rows.push((key.to_vec(), raw.to_vec()));
        Ok(())
    })?;
    check!(eq; rows, vec![(first.clone(), b"first".to_vec())]);
    check!(eq; counts.key_visits, 1);
    check!(eq; counts.encoded_bytes, u64::try_from(first.len() + 5)?);
    check!(eq; store.snapshot().visit_attachment_receipts_bounded(PREFIX, BUDGET, |_, _| Ok(()))?.key_visits, 2);
    Ok(())
}

#[test]
fn attachment_receipt_limits_refuse_before_unadmitted_callback() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    store.receipts.insert([PREFIX, b"a"].concat(), b"first")?;
    store.receipts.insert([PREFIX, b"b"].concat(), b"second")?;
    for budget in [
        ReadBudget { rows: 0, ..BUDGET },
        ReadBudget { bytes: 1, ..BUDGET },
        ReadBudget {
            record_bytes: 1,
            ..BUDGET
        },
    ] {
        let visits = Cell::new(0);
        check!(store
            .snapshot()
            .visit_attachment_receipts_bounded(PREFIX, budget, |_, _| {
                visits.set(visits.get() + 1);
                Ok(())
            })
            .is_err());
        check!(eq; visits.get(), 0);
    }
    let visits = Cell::new(0);
    let result = store.snapshot().visit_attachment_receipts_bounded(
        PREFIX,
        ReadBudget { rows: 1, ..BUDGET },
        |_, _| {
            visits.set(visits.get() + 1);
            Ok(())
        },
    );
    check!(matches!(
        result,
        Err(StoreError::BudgetExceeded {
            what: "attachment receipt rows",
            needed: 2,
            available: 1
        })
    ));
    check!(eq; visits.get(), 1);
    Ok(())
}

#[test]
fn attachment_receipt_callback_error_is_not_a_complete_prefix() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    store.receipts.insert([PREFIX, b"a"].concat(), b"first")?;
    store.receipts.insert([PREFIX, b"b"].concat(), b"second")?;
    let visits = Cell::new(0);
    let result = store
        .snapshot()
        .visit_attachment_receipts_bounded(PREFIX, BUDGET, |_, _| {
            visits.set(visits.get() + 1);
            Err(StoreError::CounterOverflow)
        });
    check!(matches!(result, Err(StoreError::CounterOverflow)));
    check!(eq; visits.get(), 1);
    Ok(())
}

#[test]
fn attachment_receipt_parent_admission_precedes_read_and_shares_callback_state() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    store.receipts.insert([PREFIX, b"a"].concat(), b"first")?;
    store.receipts.insert([PREFIX, b"b"].concat(), b"second")?;
    let mut state = (0, 0);
    let result = store.snapshot().visit_attachment_receipts_with_admission(
        PREFIX,
        ReadBudget {
            record_bytes: 0,
            ..BUDGET
        },
        &mut state,
        |state| {
            state.0 += 1;
            Err(StoreError::CounterOverflow)
        },
        |state, _, _| {
            state.1 += 1;
            Ok(())
        },
    );
    check!(matches!(result, Err(StoreError::CounterOverflow)));
    check!(eq; state, (1, 0));
    state = (0, 0);
    let result = store.snapshot().visit_attachment_receipts_with_admission(
        PREFIX,
        ReadBudget { rows: 1, ..BUDGET },
        &mut state,
        |state| {
            state.0 += 1;
            Ok(())
        },
        |state, _, _| {
            state.1 += state.0;
            Ok(())
        },
    );
    check!(matches!(
        result,
        Err(StoreError::BudgetExceeded {
            what: "attachment receipt rows",
            needed: 2,
            available: 1
        })
    ));
    check!(eq; state, (2, 1));
    Ok(())
}
