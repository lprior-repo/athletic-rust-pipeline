use super::*;

#[test]
fn empty_is_complete_with_zero_budget_and_binds_its_selector() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let view = store.snapshot();
    let zero = ReadBudget {
        rows: 0,
        bytes: 0,
        record_bytes: 0,
    };
    let empty = view.for_each_journal_root_bounded(namespace()?, zero, |_| {
        Err(StoreError::Refused {
            detail: "empty scan invoked visitor".to_string(),
        })
    })?;
    check!(eq; empty.logical_roots(), 0);
    check!(eq; empty.fingerprint().physical_rows(), 0);
    check!(eq; empty.fingerprint().encoded_bytes(), 0);
    let repeated = view.journal_namespace_fingerprint(namespace()?, zero)?;
    check!(empty.fingerprint().same_contents(&store, &repeated)?);
    let other = view.journal_namespace_fingerprint(JournalNamespace::new(PHASE, "rows/")?, zero)?;
    check!(!empty.fingerprint().same_contents(&store, &other)?);
    Ok(())
}

#[test]
fn exact_budget_proves_eof_and_one_more_row_refuses_a_certificate() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let value = br#"{"key":"row/a","payload":null}"#;
    raw(&store, "row/a", value)?;
    let exact = ReadBudget {
        rows: 1,
        bytes: encoded_bytes(&Store::journal_key(PHASE, "row/a"), value)?,
        record_bytes: value.len(),
    };
    let accepted = store
        .snapshot()
        .for_each_journal_root_bounded(namespace()?, exact, |_| Ok(()))?;
    check!(eq; accepted.logical_roots(), 1);
    check!(eq; accepted.fingerprint().physical_rows(), 1);
    check!(eq; accepted.fingerprint().encoded_bytes(), exact.bytes);
    raw(
        &store,
        "row/b",
        b"this must not be decoded by the overflow probe",
    )?;
    let mut visits = Vec::new();
    let refused = store
        .snapshot()
        .for_each_journal_root_bounded(namespace()?, exact, |key| {
            visits.push(key.to_string());
            Ok(())
        });
    check!(matches!(
        refused,
        Err(StoreError::BudgetExceeded {
            what: "journal physical rows",
            needed: 2,
            available: 1,
        })
    ));
    check!(eq; visits, vec!["row/a".to_string()]);
    Ok(())
}

#[test]
fn exact_byte_grant_minus_one_refuses_before_the_visitor() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let value = br#"{"key":"row/a","payload":null}"#;
    raw(&store, "row/a", value)?;
    let needed = encoded_bytes(&Store::journal_key(PHASE, "row/a"), value)?;
    let available = needed.checked_sub(1).ok_or(StoreError::CounterOverflow)?;
    let grant = ReadBudget {
        rows: 1,
        bytes: available,
        record_bytes: value.len(),
    };
    let mut visits = 0_u64;
    let refused = store
        .snapshot()
        .for_each_journal_root_bounded(namespace()?, grant, |_| {
            visits = visits.checked_add(1).ok_or(StoreError::CounterOverflow)?;
            Ok(())
        });
    check!(matches!(refused, Err(StoreError::BudgetExceeded {
        what: "journal encoded bytes", needed: actual_needed, available: actual_available,
    }) if actual_needed == needed && actual_available == available));
    check!(eq; visits, 0);
    Ok(())
}

#[test]
fn record_grant_minus_one_refuses_before_the_visitor() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let value = br#"{"key":"row/a","payload":null}"#;
    raw(&store, "row/a", value)?;
    let max = value
        .len()
        .checked_sub(1)
        .ok_or(StoreError::CounterOverflow)?;
    let grant = ReadBudget {
        rows: 1,
        bytes: 4096,
        record_bytes: max,
    };
    let mut visits = 0_u64;
    let refused = store
        .snapshot()
        .for_each_journal_root_bounded(namespace()?, grant, |_| {
            visits = visits.checked_add(1).ok_or(StoreError::CounterOverflow)?;
            Ok(())
        });
    check!(
        matches!(refused, Err(StoreError::RowTooLarge { bytes, max: actual_max })
        if bytes == value.len() && actual_max == max)
    );
    check!(eq; visits, 0);
    Ok(())
}

#[test]
fn admission_ceiling_is_inclusive_and_one_more_refuses() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let view = store.snapshot();
    let exact = ReadBudget {
        rows: 25_000,
        bytes: 67_108_864,
        record_bytes: MAX_JOURNAL_VALUE_BYTES,
    };
    let empty = view.for_each_journal_root_bounded(namespace()?, exact, |_| Ok(()))?;
    check!(eq; empty.logical_roots(), 0);
    check!(eq; empty.fingerprint().encoded_bytes(), 0);
    for rows in [25_001, u64::MAX] {
        let refused =
            view.journal_namespace_fingerprint(namespace()?, ReadBudget { rows, ..exact });
        check!(matches!(refused, Err(StoreError::BudgetExceeded {
            what: "journal admitted rows", needed, available: 25_000,
        }) if needed == rows));
    }
    for bytes in [67_108_865, u64::MAX] {
        let refused =
            view.journal_namespace_fingerprint(namespace()?, ReadBudget { bytes, ..exact });
        check!(matches!(refused, Err(StoreError::BudgetExceeded {
            what: "journal admitted bytes", needed, available: 67_108_864,
        }) if needed == bytes));
    }
    let one_more = MAX_JOURNAL_VALUE_BYTES
        .checked_add(1)
        .ok_or(StoreError::CounterOverflow)?;
    for record_bytes in [one_more, usize::MAX] {
        let refused = view.journal_namespace_fingerprint(
            namespace()?,
            ReadBudget {
                record_bytes,
                ..exact
            },
        );
        check!(
            matches!(refused, Err(StoreError::RowTooLarge { bytes, max })
            if bytes == record_bytes && max == MAX_JOURNAL_VALUE_BYTES)
        );
    }
    Ok(())
}

#[test]
fn oversized_physical_keys_and_values_refuse_before_root_decode() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let mut key = Store::journal_key(PHASE, "row/");
    let key_max = crate::MAX_JOURNAL_KEY_BYTES
        .checked_add(5)
        .ok_or(StoreError::CounterOverflow)?;
    let too_long = key_max.checked_add(1).ok_or(StoreError::CounterOverflow)?;
    key.resize(too_long, b'x');
    store.journal.insert(&key, b"{".as_slice())?;
    check!(
        matches!(roots(&store.snapshot()), Err(StoreError::RowTooLarge { bytes, max })
        if bytes == too_long && max == key_max)
    );
    store.journal.remove(&key)?;
    let too_large = MAX_JOURNAL_VALUE_BYTES
        .checked_add(1)
        .ok_or(StoreError::CounterOverflow)?;
    raw(&store, "row/a", &vec![b'x'; too_large])?;
    check!(
        matches!(roots(&store.snapshot()), Err(StoreError::RowTooLarge { bytes, max })
        if bytes == too_large && max == MAX_JOURNAL_VALUE_BYTES)
    );
    Ok(())
}

#[test]
fn nonempty_namespace_with_zero_rows_refuses_before_decode_or_callback() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    raw(&store, "row/a", b"{")?;
    let grant = ReadBudget {
        rows: 0,
        bytes: 0,
        record_bytes: 0,
    };
    let refused = store
        .snapshot()
        .for_each_journal_root_bounded(namespace()?, grant, |_| {
            Err(StoreError::Refused {
                detail: "unexpected callback".to_string(),
            })
        });
    check!(matches!(
        refused,
        Err(StoreError::BudgetExceeded {
            what: "journal physical rows",
            needed: 1,
            available: 0,
        })
    ));
    Ok(())
}
