use super::*;

#[test]
fn exact_namespace_budget_and_one_short_bytes_or_record_have_live_frozen_parity() -> TestResult {
    let fixture = Fixture::new()?;
    let raw = envelope("row/a", "null");
    put(&fixture.store, "row/a", raw.as_bytes())?;
    let view = fixture.store.snapshot();
    let bundle = capture_rows(&view)?;
    let frozen = FrozenSource::read(&bundle, &mut AttachmentReplayGrantV1::new())?;
    exact_and_short(&view, &raw)?;
    exact_and_short(&frozen, &raw)
}

fn exact_and_short(source: &impl RetainedJournalReadSource, raw: &str) -> TestResult {
    let bytes = u64::try_from(
        Store::journal_key(PHASE, "row/a")
            .len()
            .checked_add(raw.len())
            .ok_or(StoreError::CounterOverflow)?,
    )?;
    let exact = ReadBudget {
        rows: 1,
        bytes,
        record_bytes: raw.len(),
    };
    let mut callbacks = 0_u64;
    let complete = scan_roots(
        source,
        namespace()?,
        exact,
        &mut callbacks,
        |_| Ok(()),
        |count, _| {
            *count = count.checked_add(1).ok_or(StoreError::CounterOverflow)?;
            Ok(())
        },
    )?;
    check!(eq; callbacks, 1);
    check!(eq; complete.contents().encoded_bytes(), bytes);
    check!(eq; complete.contents().physical_rows(), 1);
    let short = bytes.checked_sub(1).ok_or(StoreError::CounterOverflow)?;
    callbacks = 0;
    let result = scan_roots(
        source,
        namespace()?,
        ReadBudget {
            bytes: short,
            ..exact
        },
        &mut callbacks,
        |_| Ok(()),
        |count, _| {
            *count = 1;
            Ok(())
        },
    );
    check!(matches!(result, Err(StoreError::BudgetExceeded {
        what: "journal encoded bytes", needed, available,
    }) if needed == bytes && available == short));
    check!(eq; callbacks, 0);
    let record_short = raw
        .len()
        .checked_sub(1)
        .ok_or(StoreError::CounterOverflow)?;
    let result = scan_roots(
        source,
        namespace()?,
        ReadBudget {
            record_bytes: record_short,
            ..exact
        },
        &mut callbacks,
        |_| Ok(()),
        |count, _| {
            *count = 1;
            Ok(())
        },
    );
    check!(matches!(result, Err(StoreError::RowTooLarge { bytes, max })
        if bytes == raw.len() && max == record_short));
    check!(eq; callbacks, 0);
    Ok(())
}

#[test]
fn row_sentinel_is_admitted_before_refusal_without_decoding_later_semantic_error() -> TestResult {
    let fixture = Fixture::new()?;
    put(
        &fixture.store,
        "row/a",
        envelope("row/a", "null").as_bytes(),
    )?;
    put(&fixture.store, "row/b", br#"{"type":null}"#)?;
    let view = fixture.store.snapshot();
    let bundle = capture_rows(&view)?;
    let frozen = FrozenSource::read(&bundle, &mut AttachmentReplayGrantV1::new())?;
    sentinel(&view)?;
    sentinel(&frozen)
}

fn sentinel(source: &impl RetainedJournalReadSource) -> TestResult {
    let mut state = (0_u64, Vec::new());
    let result = scan_roots(
        source,
        namespace()?,
        budget(1),
        &mut state,
        admit,
        |state, key| {
            state.1.push(key.to_owned());
            Ok(())
        },
    );
    check!(matches!(
        result,
        Err(StoreError::BudgetExceeded {
            what: "journal physical rows",
            needed: 2,
            available: 1,
        })
    ));
    check!(eq; state, (2, vec!["row/a".to_owned()]));
    Ok(())
}

#[test]
fn empty_captured_namespace_proves_eof_under_zero_budget() -> TestResult {
    let fixture = Fixture::new()?;
    let view = fixture.store.snapshot();
    let bundle = capture_rows(&view)?;
    let frozen = FrozenSource::read(&bundle, &mut AttachmentReplayGrantV1::new())?;
    empty(&view)?;
    empty(&frozen)
}

fn empty(source: &impl RetainedJournalReadSource) -> TestResult {
    let zero = ReadBudget {
        rows: 0,
        bytes: 0,
        record_bytes: 0,
    };
    let value = scan_roots(
        source,
        namespace()?,
        zero,
        &mut (),
        |_| Err(StoreError::CounterOverflow),
        |_, _| Err(StoreError::CounterOverflow),
    )?;
    check!(eq; value.contents().physical_rows(), 0);
    check!(eq; value.contents().encoded_bytes(), 0);
    check!(eq; value.logical_roots(), 0);
    Ok(())
}
