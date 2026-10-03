use super::{athlete, cases, school, store_of, verdicts, TestResult};
use crate::{compute_digest, reconcile_athletes, ReconcileReport};
use census_domain::model::{Gender, ReviewState};
use census_store::{StoreError, Table};

#[test]
fn same_timestamp_reconcile_refuses_receipted_decision_when_case_was_explicitly_reopened(
) -> TestResult {
    let lakeland = school("Lakeland");
    let west = school("Madison West");
    let first = athlete(&lakeland.id, "Jordan Smith", Gender::Boys, "14399169");
    let second = athlete(&west.id, "Jordan Smith", Gender::Boys, "14399169");
    let (_dir, store) = store_of(&[lakeland, west], &[first, second])?;
    let observed_at = "2026-10-01";
    let report = reconcile_athletes(&store, observed_at, false)?;
    check!(eq; (report.filed, report.decided, report.pending), (1, 1, 0));
    let resolved = cases(&store)?;
    let original_verdicts = verdicts(&store)?;
    check!(eq; resolved.len(), 1);
    check!(eq; original_verdicts.len(), 1);
    let case = resolved.first().ok_or("one resolved case")?;
    check!(eq; case.state, ReviewState::Resolved);
    let digest = compute_digest(&original_verdicts, &resolved)?;
    let operation = format!("reconcile:{observed_at}:{digest}");
    let original_receipt = store.receipt(&operation)?.ok_or("one reconcile receipt")?;
    let original_receipt_count = store.receipt_count()?;
    check!(eq; original_receipt_count, 1);
    let original_athletes = store.snapshot().athletes()?;
    let mut reopened = case.clone();
    reopened.state = ReviewState::Pending;
    store.replace_many(Table::ReviewCases, std::slice::from_ref(&reopened))?;
    check!(eq; cases(&store)?, vec![reopened.clone()]);
    let repeated = reconcile_athletes(&store, observed_at, false);
    check!(eq; cases(&store)?, vec![reopened]);
    check!(eq; verdicts(&store)?, original_verdicts);
    check!(eq; store.receipt_count()?, original_receipt_count);
    check!(eq; store.receipt(&operation)?, Some(original_receipt));
    check!(eq; store.snapshot().athletes()?, original_athletes);
    check!(
        matches!(repeated, Err(StoreError::Invariant { .. })),
        "a reopened Pending case cannot acknowledge the original resolved receipt: {repeated:?}"
    );
    Ok(())
}

#[test]
fn exact_reconcile_repetition_preserves_standing_case_verdict_and_receipt() -> TestResult {
    let lakeland = school("Lakeland");
    let west = school("Madison West");
    let first = athlete(&lakeland.id, "Jordan Smith", Gender::Boys, "14399169");
    let second = athlete(&west.id, "Jordan Smith", Gender::Boys, "14399169");
    let (_dir, store) = store_of(&[lakeland, west], &[first, second])?;
    let observed_at = "2026-10-01";
    let report = reconcile_athletes(&store, observed_at, false)?;
    check!(eq; (report.filed, report.decided, report.pending), (1, 1, 0));
    let resolved = cases(&store)?;
    let original_verdicts = verdicts(&store)?;
    check!(eq; resolved.len(), 1);
    check!(eq; original_verdicts.len(), 1);
    check!(eq; resolved.first().map(|case| case.state), Some(ReviewState::Resolved));
    let digest = compute_digest(&original_verdicts, &resolved)?;
    let operation = format!("reconcile:{observed_at}:{digest}");
    let original_receipt = store.receipt(&operation)?.ok_or("one reconcile receipt")?;
    let original_receipt_count = store.receipt_count()?;
    check!(eq; original_receipt_count, 1);
    let repeated = reconcile_athletes(&store, observed_at, false)?;
    check!(eq; repeated, ReconcileReport { rows: 2, objects: 1, filed: 0, decided: 0, pending: 0, aliases: 0, held: 1 });
    check!(eq; cases(&store)?, resolved);
    check!(eq; verdicts(&store)?, original_verdicts);
    check!(eq; store.receipt_count()?, original_receipt_count);
    check!(eq; store.receipt(&operation)?, Some(original_receipt));
    Ok(())
}
