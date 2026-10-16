use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn exceeded<T>(
    result: StoreResult<T>,
    label: &'static str,
    needed: usize,
    available: usize,
) -> TestResult {
    let needed = u64::try_from(needed)?;
    let available = u64::try_from(available)?;
    check!(matches!(result, Err(StoreError::BudgetExceeded {
        what, needed: actual_needed, available: actual_available,
    }) if what == label && actual_needed == needed && actual_available == available));
    Ok(())
}

#[test]
fn private_row_counter_admits_exact_limit_and_refuses_one_more_without_allocating_rows(
) -> TestResult {
    check!(eq; MAX_ROWS, 40_000);
    let mut grant = AttachmentReplayGrantV1::new();
    let mut state = State::new(&mut grant);
    state.rows = MAX_ROWS.checked_sub(1).ok_or(StoreError::CounterOverflow)?;
    state.admit_row()?;
    check!(eq; state.rows, MAX_ROWS);
    exceeded(state.admit_row(), "frozen archive rows", 40_001, 40_000)
}

#[test]
fn private_prefix_counter_admits_fixed_groups_plus_limit_and_refuses_one_more() -> TestResult {
    check!(eq; MAX_PREFIXES, 40_002);
    let mut grant = AttachmentReplayGrantV1::new();
    let mut state = State::new(&mut grant);
    state.prefixes = MAX_PREFIXES
        .checked_sub(1)
        .ok_or(StoreError::CounterOverflow)?;
    state.prefix()?;
    check!(eq; state.prefixes, MAX_PREFIXES);
    exceeded(state.prefix(), "frozen archive prefixes", 40_003, 40_002)
}

#[test]
fn private_physical_byte_counter_keeps_the_exact_limit_and_first_one_over() -> TestResult {
    check!(eq; MAX_ARCHIVE_BYTES, 536_870_912);
    let mut grant = AttachmentReplayGrantV1::new();
    let mut state = State::new(&mut grant);
    state.raw_bytes = MAX_ARCHIVE_BYTES
        .checked_sub(1)
        .ok_or(StoreError::CounterOverflow)?;
    state.row(0, 1)?;
    check!(eq; state.raw_bytes, MAX_ARCHIVE_BYTES);
    exceeded(
        state.row(0, 1),
        "frozen archive physical bytes",
        536_870_913,
        536_870_912,
    )
}

#[test]
fn private_key_byte_counter_admits_exact_4101_byte_rows_and_one_over_refuses() -> TestResult {
    let maximum = 40_000_usize
        .checked_mul(4101)
        .ok_or(StoreError::CounterOverflow)?;
    let mut grant = AttachmentReplayGrantV1::new();
    let mut state = State::new(&mut grant);
    state.key_bytes = maximum
        .checked_sub(4101)
        .ok_or(StoreError::CounterOverflow)?;
    state.row(4101, 0)?;
    check!(eq; state.key_bytes, maximum);
    exceeded(
        state.row(1, 0),
        "frozen archive key bytes",
        maximum.checked_add(1).ok_or(StoreError::CounterOverflow)?,
        maximum,
    )
}

#[test]
fn private_metadata_counter_includes_rows_prefixes_and_keys_at_exact_boundary() -> TestResult {
    let row_bytes = 40_000_usize
        .checked_mul(std::mem::size_of::<Row<'_>>())
        .ok_or(StoreError::CounterOverflow)?;
    let prefix_bytes = 40_002_usize
        .checked_mul(std::mem::size_of::<Prefix<'_>>())
        .ok_or(StoreError::CounterOverflow)?;
    let maximum = 40_000_usize
        .checked_mul(4101)
        .and_then(|n| n.checked_add(row_bytes))
        .and_then(|n| n.checked_add(prefix_bytes))
        .ok_or(StoreError::CounterOverflow)?;
    let mut grant = AttachmentReplayGrantV1::new();
    let mut state = State::new(&mut grant);
    state.metadata_bytes = maximum.checked_sub(1).ok_or(StoreError::CounterOverflow)?;
    state.metadata(1)?;
    check!(eq; state.metadata_bytes, maximum);
    exceeded(
        state.metadata(1),
        "frozen archive index bytes",
        maximum.checked_add(1).ok_or(StoreError::CounterOverflow)?,
        maximum,
    )
}

#[test]
fn original_grant_exhaustion_precedes_private_row_and_prefix_counter_changes() -> TestResult {
    for prefix in [false, true] {
        let mut grant = AttachmentReplayGrantV1::new();
        grant.charge_steps(8192)?;
        let mut state = State::new(&mut grant);
        let result = if prefix {
            state.prefix()
        } else {
            state.admit_row()
        };
        check!(matches!(
            result,
            Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
                EvidenceSubjectAttachmentError::Capacity {
                    resource: "aggregate traversal steps",
                    needed: 8193,
                    available: 8192,
                }
            )))
        ));
        check!(eq; state.rows, 0);
        check!(eq; state.prefixes, 0);
        check!(matches!(
            state.grant.charge_steps(0),
            Err(EvidenceSubjectAttachmentError::Poisoned)
        ));
    }
    Ok(())
}

#[test]
fn private_counter_overflow_and_first_error_are_not_replaced_by_parse_wrappers() -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    let mut state = State::new(&mut grant);
    state.raw_bytes = usize::MAX;
    check!(matches!(state.row(0, 1), Err(StoreError::CounterOverflow)));
    let first: serde_json::Error = state.fail(StoreError::CounterOverflow);
    let second: serde_json::Error =
        state.fail(Error::Corrupt("secondary synthetic failure").into());
    check!(first.is_data());
    check!(second.is_data());
    check!(matches!(
        state.failure.take(),
        Some(StoreError::CounterOverflow)
    ));
    Ok(())
}
