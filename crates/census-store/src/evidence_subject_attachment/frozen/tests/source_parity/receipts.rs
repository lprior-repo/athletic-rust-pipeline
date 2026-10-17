use super::*;

#[test]
fn complete_receipt_prefix_and_narrower_selection_match_exact_raw_rows() -> TestResult {
    let fixture = Fixture::new()?;
    for suffix in [b"a", b"b"] {
        fixture
            .store
            .receipts
            .insert([PREFIX, suffix].concat(), br#"{"v":1e0}"#)?;
    }
    fixture.store.receipts.insert(b"unrelated", br#"{"v":2}"#)?;
    let view = fixture.store.snapshot();
    let bundle = capture_rows(&view)?;
    let frozen = FrozenSource::read(&bundle, &mut AttachmentReplayGrantV1::new())?;
    for prefix in [PREFIX.to_vec(), [PREFIX, b"a"].concat()] {
        check!(eq; receipt_rows(&view, &prefix)?, receipt_rows(&frozen, &prefix)?);
    }
    let (rows, count, bytes) = receipt_rows(&frozen, PREFIX)?;
    check!(eq; count, 2);
    check!(eq; rows, vec![
        ([PREFIX, b"a"].concat(), br#"{"v":1e0}"#.to_vec()),
        ([PREFIX, b"b"].concat(), br#"{"v":1e0}"#.to_vec()),
    ]);
    let expected = PREFIX
        .len()
        .checked_add(1)
        .and_then(|n| n.checked_add(9))
        .and_then(|n| n.checked_mul(2))
        .ok_or(StoreError::CounterOverflow)?;
    check!(eq; bytes, u64::try_from(expected)?);
    Ok(())
}

type ReceiptRows = (Vec<(Vec<u8>, Vec<u8>)>, u64, u64);

fn receipt_rows(source: &impl RetainedJournalReadSource, prefix: &[u8]) -> TestResult<ReceiptRows> {
    let mut rows = Vec::new();
    let counts = source.visit_receipt_prefix(
        prefix,
        budget(16),
        &mut rows,
        |_| Ok(()),
        |rows, key, value| {
            rows.push((key.to_vec(), value.to_vec()));
            Ok(())
        },
    )?;
    Ok((rows, counts.key_visits, counts.encoded_bytes))
}

#[test]
fn receipt_parent_admission_and_overflow_sentinel_precede_unadmitted_values() -> TestResult {
    let fixture = Fixture::new()?;
    for suffix in [b"a", b"b"] {
        fixture
            .store
            .receipts
            .insert([PREFIX, suffix].concat(), b"null")?;
    }
    let view = fixture.store.snapshot();
    let bundle = capture_rows(&view)?;
    let frozen = FrozenSource::read(&bundle, &mut AttachmentReplayGrantV1::new())?;
    receipt_admission(&view)?;
    receipt_admission(&frozen)
}

fn receipt_admission(source: &impl RetainedJournalReadSource) -> TestResult {
    let mut state = (0_u64, 0_u64);
    let result = source.visit_receipt_prefix(
        PREFIX,
        ReadBudget {
            record_bytes: 0,
            ..budget(2)
        },
        &mut state,
        |state| {
            admit(state)?;
            Err(StoreError::CounterOverflow)
        },
        |state, _, _| {
            state.1 = 1;
            Ok(())
        },
    );
    check!(matches!(result, Err(StoreError::CounterOverflow)));
    check!(eq; state, (1, 0));
    state = (0, 0);
    let result =
        source.visit_receipt_prefix(PREFIX, budget(1), &mut state, admit, |state, _, _| {
            state.1 = state.0;
            Ok(())
        });
    check!(matches!(
        result,
        Err(StoreError::BudgetExceeded {
            what: "attachment receipt rows",
            needed: 2,
            available: 1,
        })
    ));
    check!(eq; state, (2, 1));
    Ok(())
}

#[test]
fn receipt_exact_and_one_short_admissions_preserve_the_callback_boundary() -> TestResult {
    let fixture = Fixture::new()?;
    fixture
        .store
        .receipts
        .insert([PREFIX, b"a"].concat(), b"null")?;
    let view = fixture.store.snapshot();
    let bundle = capture_rows(&view)?;
    let frozen = FrozenSource::read(&bundle, &mut AttachmentReplayGrantV1::new())?;
    receipt_limits(&view)?;
    receipt_limits(&frozen)
}

fn receipt_limits(source: &impl RetainedJournalReadSource) -> TestResult {
    let bytes = u64::try_from(
        PREFIX
            .len()
            .checked_add(5)
            .ok_or(StoreError::CounterOverflow)?,
    )?;
    let exact = ReadBudget {
        rows: 1,
        bytes,
        record_bytes: 4,
    };
    let mut visits = 0;
    let counts = source.visit_receipt_prefix(
        PREFIX,
        exact,
        &mut visits,
        |_| Ok(()),
        |visits, _, _| {
            *visits = 1;
            Ok(())
        },
    )?;
    check!(eq; visits, 1);
    check!(eq; counts.encoded_bytes, bytes);
    visits = 0;
    let short = bytes.checked_sub(1).ok_or(StoreError::CounterOverflow)?;
    let result = source.visit_receipt_prefix(
        PREFIX,
        ReadBudget {
            bytes: short,
            ..exact
        },
        &mut visits,
        |_| Ok(()),
        |visits, _, _| {
            *visits = 1;
            Ok(())
        },
    );
    check!(matches!(result, Err(StoreError::BudgetExceeded {
        what: "capture encoded bytes", needed, available,
    }) if needed == bytes && available == short));
    check!(eq; visits, 0);
    let result = source.visit_receipt_prefix(
        PREFIX,
        ReadBudget {
            record_bytes: 3,
            ..exact
        },
        &mut visits,
        |_| Ok(()),
        |visits, _, _| {
            *visits = 1;
            Ok(())
        },
    );
    check!(matches!(
        result,
        Err(StoreError::RowTooLarge { bytes: 4, max: 3 })
    ));
    check!(eq; visits, 0);
    Ok(())
}

#[test]
fn receipt_visitor_error_and_old_snapshot_remain_exact_after_live_append() -> TestResult {
    let fixture = Fixture::new()?;
    fixture
        .store
        .receipts
        .insert([PREFIX, b"a"].concat(), b"null")?;
    let held = fixture.store.snapshot();
    let bundle = capture_rows(&held)?;
    let frozen = FrozenSource::read(&bundle, &mut AttachmentReplayGrantV1::new())?;
    fixture
        .store
        .receipts
        .insert([PREFIX, b"b"].concat(), b"true")?;
    check!(eq; receipt_rows(&held, PREFIX)?, receipt_rows(&frozen, PREFIX)?);
    check!(eq; receipt_rows(&fixture.store.snapshot(), PREFIX)?.1, 2);
    receipt_visitor(&held)?;
    receipt_visitor(&frozen)
}

fn receipt_visitor(source: &impl RetainedJournalReadSource) -> TestResult {
    let mut count = 0;
    let result = source.visit_receipt_prefix(
        PREFIX,
        budget(2),
        &mut count,
        |_| Ok(()),
        |count, _, _| {
            *count = 1;
            Err(StoreError::CounterOverflow)
        },
    );
    check!(matches!(result, Err(StoreError::CounterOverflow)));
    check!(eq; count, 1);
    Ok(())
}
