use std::ops::ControlFlow;

use census_domain::model::{RetainedConflict, ReviewCase, ReviewState};

use crate::{Store, StoreError, Table};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn keyed_review_reads_and_cursor_stop_preserve_captured_generation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let mut cases: Vec<_> = ["a", "a:b", "a\0b", "🦀", "z"]
        .into_iter()
        .map(|id| ReviewCase::pending("fixture", id, "subject", "detail"))
        .collect();
    cases.sort_by(|left, right| left.id.cmp(&right.id));
    store.replace_many(Table::ReviewCases, &cases)?;
    let snapshot = store.snapshot();
    for case in &cases {
        check!(eq; snapshot.get_merged::<ReviewCase>(Table::ReviewCases, &case.id)?, Some(case.clone()));
    }
    check!(eq; snapshot.get_merged::<ReviewCase>(Table::ReviewCases, "absent")?, None);
    let mut cursor = None;
    let mut visited = Vec::new();
    for _ in &cases {
        let after = cursor.clone();
        let count = snapshot.for_each_merged_after(
            Table::ReviewCases,
            after.as_deref(),
            |case: ReviewCase| {
                cursor = Some(case.id.clone());
                visited.push(case);
                Ok(ControlFlow::Break(()))
            },
        )?;
        check!(eq; count, 1);
    }
    check!(eq; serde_json::to_vec(&visited)?, serde_json::to_vec(&cases)?);
    let mut changed = cases.first().ok_or("first case")?.clone();
    changed.state = ReviewState::Resolved;
    store.replace(Table::ReviewCases, &changed)?;
    check!(eq; snapshot.get_merged::<ReviewCase>(Table::ReviewCases, &changed.id)?, cases.first().cloned());
    check!(eq; store.snapshot().get_merged::<ReviewCase>(Table::ReviewCases, &changed.id)?, Some(changed));
    Ok(())
}

#[test]
fn keyed_reads_merge_only_the_exact_observation_identifier() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let first = RetainedConflict::new("fixture", "a", "subject", "first");
    let latest = RetainedConflict::new("fixture", "a", "subject", "last");
    store.append(Table::Schools, &first)?;
    store.append(
        Table::Schools,
        &RetainedConflict::new("fixture", "a\0b", "subject", "foreign"),
    )?;
    let snapshot = store.snapshot();
    store.append(Table::Schools, &latest)?;
    check!(eq; snapshot.get_merged::<RetainedConflict>(Table::Schools, &first.id)?, Some(first.clone()));
    check!(eq; store.snapshot().get_merged::<RetainedConflict>(Table::Schools, &first.id)?, Some(latest));
    check!(
        matches!(snapshot.for_each_merged_after::<RetainedConflict>(Table::Schools, None, |_| Ok(ControlFlow::Continue(()))), Err(StoreError::Invariant { detail }) if detail == "cursor reads require a generation-partitioned table")
    );
    Ok(())
}
