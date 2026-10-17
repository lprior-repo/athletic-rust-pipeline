mod fixtures;
use crate::{Store, StoreError, Table, MAX_OBSERVATION_BATCH_BYTES, MAX_OBSERVATION_BATCH_ROWS};
use census_domain::model::CanonicalSchool;
use fixtures::{
    assert_reopened_without_writes, assert_retained, child, offered_bytes, padded_school,
    receipt_bytes, schools, smallest_offered_row, staged_batch,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const OUTER_OPERATION: &str = "offer-limits/outer";
const OUTER_DIGEST: &str = "offer-limits-outer-digest";
const CHILD_OPERATION: &str = "offer-limits/child";
const CHILD_DIGEST: &str = "offer-limits-child-digest";
const SECOND_CHILD_OPERATION: &str = "offer-limits/second-child";
const SECOND_CHILD_DIGEST: &str = "offer-limits-second-child-digest";

#[test]
fn an_exactly_full_offered_row_count_is_accepted() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let outer_rows = schools("outer", 1)?;
    let child_count = MAX_OBSERVATION_BATCH_ROWS
        .checked_sub(1)
        .ok_or("child rows")?;
    let child_rows = schools("child", child_count)?;
    let outer = staged_batch(&store, &outer_rows)?;
    let application = outer.commit_once_with_effects(
        OUTER_OPERATION,
        OUTER_DIGEST,
        vec![child(&store, CHILD_OPERATION, CHILD_DIGEST, &child_rows)?],
    )?;
    check!(application.written());
    check!(eq; store.receipt_count()?, 2);
    check!(eq; store.count(Table::Schools)?, u64::try_from(MAX_OBSERVATION_BATCH_ROWS)?);
    check!(eq; store.walk_table(Table::Schools)?.rows, u64::try_from(MAX_OBSERVATION_BATCH_ROWS)?);
    check!(eq; store.scan::<CanonicalSchool>(Table::Schools)?.len(), MAX_OBSERVATION_BATCH_ROWS);
    Ok(())
}

#[test]
fn an_offered_row_count_one_over_the_limit_refuses_without_any_write() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let sequence = store.snapshot().sequence();
    let generation = store.evidence_generation();
    let outer_rows = schools("outer", 1)?;
    let child_rows = schools("child", MAX_OBSERVATION_BATCH_ROWS)?;
    let outer = staged_batch(&store, &outer_rows)?;
    let result = outer.commit_once_with_effects(
        OUTER_OPERATION,
        OUTER_DIGEST,
        vec![child(&store, CHILD_OPERATION, CHILD_DIGEST, &child_rows)?],
    );
    check!(
        matches!(
            result,
            Err(StoreError::BudgetExceeded {
                what: "observation batch rows",
                needed: 4097,
                available: 4096
            })
        ),
        "{result:?}"
    );
    assert_retained(&store, sequence, generation, 0)?;
    check!(store.receipt(OUTER_OPERATION)?.is_none());
    check!(store.receipt(CHILD_OPERATION)?.is_none());
    check!(eq; store.receipt_count()?, 0);
    drop(store);
    assert_reopened_without_writes(directory.path(), &[OUTER_OPERATION, CHILD_OPERATION])?;
    Ok(())
}

#[test]
fn a_repeated_child_still_counts_in_the_offered_row_limit() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let retained_rows = schools("retained", MAX_OBSERVATION_BATCH_ROWS)?;
    let precondition = staged_batch(&store, &retained_rows)?;
    let preconditioned = precondition.commit_once(CHILD_OPERATION, CHILD_DIGEST)?;
    check!(preconditioned.written());
    check!(eq; preconditioned.receipt().appended, u64::try_from(MAX_OBSERVATION_BATCH_ROWS)?);
    let retained_receipt = store.receipt(CHILD_OPERATION)?.ok_or("child receipt")?;
    let retained_bytes = receipt_bytes(&store, CHILD_OPERATION)?;
    let sequence = store.snapshot().sequence();
    let generation = store.evidence_generation();
    let outer_rows = schools("outer", 1)?;
    let outer = staged_batch(&store, &outer_rows)?;
    let result = outer.commit_once_with_effects(
        OUTER_OPERATION,
        OUTER_DIGEST,
        vec![child(
            &store,
            CHILD_OPERATION,
            CHILD_DIGEST,
            &retained_rows,
        )?],
    );
    check!(
        matches!(
            result,
            Err(StoreError::BudgetExceeded {
                what: "observation batch rows",
                needed: 4097,
                available: 4096
            })
        ),
        "{result:?}"
    );
    assert_retained(&store, sequence, generation, MAX_OBSERVATION_BATCH_ROWS)?;
    check!(store.receipt(OUTER_OPERATION)?.is_none());
    check!(eq; store.receipt(CHILD_OPERATION)?, Some(retained_receipt));
    check!(eq; receipt_bytes(&store, CHILD_OPERATION)?, retained_bytes);
    check!(eq; store.receipt_count()?, 1);
    drop(store);
    let reopened = Store::open(directory.path())?;
    let retained = u64::try_from(MAX_OBSERVATION_BATCH_ROWS)?;
    check!(eq; reopened.count(Table::Schools)?, retained);
    check!(eq; reopened.walk_table(Table::Schools)?.rows, retained);
    check!(eq; reopened.scan::<CanonicalSchool>(Table::Schools)?.len(), MAX_OBSERVATION_BATCH_ROWS);
    check!(reopened.receipt(OUTER_OPERATION)?.is_none());
    check!(eq; receipt_bytes(&reopened, CHILD_OPERATION)?, retained_bytes);
    check!(eq; reopened.receipt_count()?, 1);
    Ok(())
}

#[test]
fn combined_child_bytes_over_the_batch_budget_refuse_without_any_write() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let sequence = store.snapshot().sequence();
    let generation = store.evidence_generation();
    let outer_rows: Vec<CanonicalSchool> = (0..1).map(padded_school).collect::<Result<_, _>>()?;
    let first_rows: Vec<CanonicalSchool> = (1..12).map(padded_school).collect::<Result<_, _>>()?;
    let second_rows: Vec<CanonicalSchool> =
        (12..19).map(padded_school).collect::<Result<_, _>>()?;
    let outer_bytes = offered_bytes(&outer_rows)?;
    let first_bytes = offered_bytes(&first_rows)?;
    let second_bytes = offered_bytes(&second_rows)?;
    let expected = outer_bytes
        .checked_add(first_bytes)
        .and_then(|total| total.checked_add(second_bytes))
        .ok_or("offered byte sum overflow")?;
    let smallest = smallest_offered_row(&outer_rows, &first_rows, &second_rows)?;
    check!(
        first_bytes <= MAX_OBSERVATION_BATCH_BYTES,
        "first child bytes {first_bytes}"
    );
    check!(
        second_bytes <= MAX_OBSERVATION_BATCH_BYTES,
        "second child bytes {second_bytes}"
    );
    check!(
        expected > MAX_OBSERVATION_BATCH_BYTES,
        "combined offered bytes {expected}"
    );
    check!(
        expected
            .checked_sub(smallest)
            .ok_or("smallest offered row")?
            <= MAX_OBSERVATION_BATCH_BYTES,
        "offered bytes past any single row must fit: {expected} minus {smallest}"
    );
    let outer = staged_batch(&store, &outer_rows)?;
    let result = outer.commit_once_with_effects(
        OUTER_OPERATION,
        OUTER_DIGEST,
        vec![
            child(&store, CHILD_OPERATION, CHILD_DIGEST, &first_rows)?,
            child(
                &store,
                SECOND_CHILD_OPERATION,
                SECOND_CHILD_DIGEST,
                &second_rows,
            )?,
        ],
    );
    check!(
        matches!(
            result,
            Err(StoreError::BudgetExceeded {
                what: "observation batch bytes",
                needed,
                available
            }) if available == u64::try_from(MAX_OBSERVATION_BATCH_BYTES)?
                && needed == u64::try_from(expected)?
        ),
        "{result:?}"
    );
    assert_retained(&store, sequence, generation, 0)?;
    check!(store.receipt(OUTER_OPERATION)?.is_none());
    check!(store.receipt(CHILD_OPERATION)?.is_none());
    check!(store.receipt(SECOND_CHILD_OPERATION)?.is_none());
    check!(eq; store.receipt_count()?, 0);
    drop(store);
    assert_reopened_without_writes(
        directory.path(),
        &[OUTER_OPERATION, CHILD_OPERATION, SECOND_CHILD_OPERATION],
    )?;
    Ok(())
}
