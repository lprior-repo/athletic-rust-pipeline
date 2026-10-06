use super::*;
use crate::tests::{derived, generation_rows, DerivedRow, TestResult};
use crate::{generation, meta};

fn publish(store: &Store, rows: &[DerivedRow], operation: &str) -> TestResult {
    let mut stage = store.stage_derived()?;
    stage.replace_many(Table::ReviewCases, rows)?;
    stage.publish(operation, "digest-one")?;
    Ok(())
}

#[test]
fn a_budget_smaller_than_one_chunk_leaves_the_rest_for_the_next_pass() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let rows: Vec<DerivedRow> = (0..6)
        .map(|index| derived(&format!("case-{index}"), 1))
        .collect();
    publish(&store, &rows, "publish:index")?;
    let prefix = derived_generation_prefix(Table::ReviewCases, 1);
    let mut budget = 2_u64;
    let mut removed = 0_u64;
    let complete = delete_prefix(&store, &prefix, &mut budget, &mut removed)?;
    check!(
        !complete,
        "a pass that ran out of budget must not report the prefix as reclaimed"
    );
    check!(eq; removed, 2);
    check!(
        eq;
        generation_rows(&store, Table::ReviewCases, 1)?,
        4,
        "rows past the budget must stay for the next pass"
    );
    let mut budget = 64_u64;
    let mut again = 0_u64;
    let complete = delete_prefix(&store, &prefix, &mut budget, &mut again)?;
    check!(complete, "the next pass with room must finish the prefix");
    check!(eq; again, 4);
    check!(eq; generation_rows(&store, Table::ReviewCases, 1)?, 0);
    Ok(())
}

#[test]
fn publication_reclaims_the_generation_the_pointer_left_behind() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let first: Vec<DerivedRow> = (0..4)
        .map(|index| derived(&format!("case-{index}"), 1))
        .collect();
    publish(&store, &first, "publish:first")?;
    publish(&store, &[derived("case-0", 2)], "publish:second")?;
    check!(eq; store.derived_generation(), 2);
    check!(
        eq;
        generation_rows(&store, Table::ReviewCases, 1)?,
        0,
        "the superseded generation must be gone once the pointer has moved"
    );
    check!(eq; generation_rows(&store, Table::ReviewCases, 2)?, 1);
    check!(
        eq;
        meta::get_u64(&store.meta, generation::DERIVED_RECLAIM_FROM)?.map_or(0, |from| from),
        2,
        "the reclaim pointer follows the generations that are gone"
    );
    Ok(())
}

#[test]
fn reclaim_of_a_store_with_nothing_superseded_reports_complete() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let reclaimed = store.reclaim_derived_generations(u64::MAX)?;
    check!(reclaimed.complete);
    check!(eq; reclaimed.rows, 0);
    Ok(())
}
