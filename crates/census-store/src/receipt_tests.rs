use super::*;
use census_domain::model::*;
use census_domain::UsJurisdiction;
use chrono::NaiveDate;

use super::clock::{Clock, SystemClock};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn school(name: &str) -> CanonicalSchool {
    CanonicalSchool::new(UsJurisdiction::Wisconsin, name, normalize_name(name), None).0
}

fn observed(name: &str, source: &str, at: &str) -> CanonicalSchool {
    let mut row = school(name);
    row.evidence
        .push(Evidence::parsed(SourceRef::id(source), at));
    row
}

fn page() -> Vec<CanonicalSchool> {
    vec![
        observed("Abbotsford", "wiaa_schools", "2026-09-19"),
        observed("Colby", "wiaa_schools", "2026-09-19"),
    ]
}

fn rows(store: &Store, table: Table) -> TestResult<u64> {
    Ok(store
        .stats()?
        .tables
        .into_iter()
        .find(|(name, _)| name == table.file())
        .map(|(_, count)| count)
        .ok_or("table row count")?)
}

fn counter(store: &Store, table: Table) -> TestResult<u64> {
    Ok(store
        .stats()?
        .appended
        .into_iter()
        .find(|(name, _)| name == table.file())
        .map(|(_, next)| next)
        .ok_or("table append counter")?)
}

fn apply(
    store: &Store,
    operation: &str,
    digest: &str,
    journal_key: &str,
) -> StoreResult<Application> {
    let mut batch = store.write_batch();
    batch.append_many(Table::Schools, &page())?;
    batch.journal_done("wiaa_schools", journal_key, &operation)?;
    batch.commit_once(operation, digest)
}

fn day(offset: i64) -> TestResult<String> {
    let today = NaiveDate::parse_from_str(&SystemClock.today(), "%Y-%m-%d")?;
    let moved = if offset < 0 {
        today - chrono::Days::new(u64::try_from(-offset)?)
    } else {
        today + chrono::Days::new(u64::try_from(offset)?)
    };
    Ok(moved.to_string())
}

fn frontier(at: &str) -> ReplayFrontier {
    ReplayFrontier {
        at: at.to_string(),
        witnesses: Vec::new(),
    }
}

fn witness(invocation: &str, operation: &str) -> ReplayWitness {
    ReplayWitness {
        invocation: invocation.to_string(),
        operation: operation.to_string(),
    }
}

#[test]
fn a_replay_of_one_operation_writes_nothing() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let first = apply(
        &store,
        "wiaa_schools_wi:w39:schools:0",
        "digest-a",
        "unit-0",
    )?;
    let receipt = match &first {
        Application::Written(receipt) => receipt,
        other => {
            return Err(format!(
                "the first application of an operation writes its receipt: {other:?}"
            )
            .into())
        }
    };
    check!(eq; receipt.appended, 2);
    check!(eq; receipt.operation, "wiaa_schools_wi:w39:schools:0");
    check!(eq; rows(&store, Table::Schools)?, 2);
    let repeat = apply(
        &store,
        "wiaa_schools_wi:w39:schools:0",
        "digest-a",
        "unit-0",
    )?;
    let standing = match &repeat {
        Application::Repeated(standing) => standing,
        other => {
            return Err(
                format!("a repeat of an applied operation is not written again: {other:?}").into(),
            )
        }
    };
    check!(eq; repeat.appended(), 0, "a replay appends nothing, so a caller summing replies cannot count the page twice");
    check!(eq; standing, receipt, "the replay answers with the receipt the first application left");
    check!(eq; rows(&store, Table::Schools)?, 2, "no row was written twice");
    check!(eq; store.receipt_count()?, 1, "one operation, one receipt — the replay did not add another");
    Ok(())
}

#[test]
fn one_operation_id_cannot_name_two_payloads() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    apply(
        &store,
        "wiaa_schools_wi:w39:schools:0",
        "digest-a",
        "unit-0",
    )?;
    let error = match apply(
        &store,
        "wiaa_schools_wi:w39:schools:0",
        "digest-b",
        "unit-0",
    ) {
        Err(error) => error,
        Ok(outcome) => {
            return Err(
                format!("a changed payload under an applied id is refused: {outcome:?}").into(),
            )
        }
    };
    check!(
        matches!(error, StoreError::Invariant { .. }),
        "the refusal is an invariant violation, which is terminal: {error:?}"
    );
    let message = error.to_string();
    check!(
        message.contains("wiaa_schools_wi:w39:schools:0"),
        "the refusal names the operation: {message}"
    );
    check!(
        message.contains("digest-a") && message.contains("digest-b"),
        "the refusal names both digests: {message}"
    );
    check!(eq; rows(&store, Table::Schools)?, 2, "the refused page wrote no rows");
    check!(eq; store.receipt_count()?, 1, "the refused page wrote no receipt");
    let receipt = store
        .receipt("wiaa_schools_wi:w39:schools:0")?
        .ok_or("standing receipt")?;
    check!(eq; receipt.digest, "digest-a", "the standing receipt is the first one");
    Ok(())
}

#[test]
fn the_receipt_outlives_the_process_that_wrote_it() -> TestResult {
    let dir = tempfile::tempdir()?;
    let operation = "wiaa_schools_wi:w39:schools:0";
    {
        let store = Store::open(dir.path())?;
        apply(&store, operation, "digest-a", "unit-0")?;
    }
    let store = Store::open(dir.path())?;
    let receipt = store.receipt(operation)?.ok_or("persisted receipt")?;
    check!(eq; receipt.appended, 2);
    let repeat = apply(&store, operation, "digest-a", "unit-0")?;
    check!(matches!(repeat, Application::Repeated(_)), "{repeat:?}");
    check!(eq; rows(&store, Table::Schools)?, 2);
    Ok(())
}

#[test]
fn a_replay_moves_no_counter_and_writes_no_second_journal_entry() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    apply(
        &store,
        "wiaa_schools_wi:w39:schools:0",
        "digest-a",
        "unit-0",
    )?;
    let rows_before = rows(&store, Table::Schools)?;
    let counter_before = counter(&store, Table::Schools)?;
    let journal = store.journal_keys("wiaa_schools")?;
    check!(
        journal.contains("unit-0"),
        "the operation's marker was written"
    );
    let repeat = apply(
        &store,
        "wiaa_schools_wi:w39:schools:0",
        "digest-a",
        "unit-0",
    )?;
    check!(matches!(repeat, Application::Repeated(_)), "{repeat:?}");
    check!(eq; counter(&store, Table::Schools)?, counter_before);
    check!(eq; rows(&store, Table::Schools)?, rows_before);
    check!(eq; store.journal_keys("wiaa_schools")?, journal, "the marker a replay would rewrite is the one already standing");
    Ok(())
}

#[test]
fn an_operation_with_no_rows_still_has_a_receipt() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let mut batch = store.write_batch();
    batch.append_many(Table::Schools, &Vec::<CanonicalSchool>::new())?;
    check!(batch.is_empty(), "an empty page buffers nothing");
    let outcome = batch.commit_once("wiaa_schools_wi:w39:schools:empty", "digest-e")?;
    check!(eq; outcome.appended(), 0, "an operation with no rows appends none");
    check!(
        matches!(outcome, Application::Written(_)),
        "and is still applied"
    );
    check!(eq; store.receipt_count()?, 1);
    let mut batch = store.write_batch();
    batch.append_many(Table::Schools, &page())?;
    let repeat = batch.commit_once("wiaa_schools_wi:w39:schools:empty", "digest-e")?;
    let receipt = match repeat {
        Application::Repeated(receipt) => receipt,
        other => {
            return Err(format!(
                "the empty operation is a repeat, so its page is not written: {other:?}"
            )
            .into())
        }
    };
    check!(eq; receipt.digest, "digest-e");
    check!(eq; rows(&store, Table::Schools)?, 0, "the repeat wrote no rows");
    Ok(())
}

#[test]
fn receipts_older_than_the_policy_day_are_removed_and_newer_ones_kept() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let yesterday = day(-1)?;
    let tomorrow = day(1)?;
    apply(&store, "old-operation", "digest-old", "unit-old")?;
    let kept = store.prune_receipts(&frontier(&yesterday))?;
    check!(
        eq;
        kept,
        Pruned {
            frontier_advanced: true,
            ..Pruned::default()
        },
        "nothing older than yesterday"
    );
    check!(store.receipt("old-operation")?.is_some());
    let pruned = store.prune_receipts(&frontier(&tomorrow))?;
    check!(eq; pruned.removed, 1);
    check!(eq; pruned.undated, 0);
    check!(eq; pruned.witnesses_kept, 0, "a frontier with no witnesses retains none");
    check!(eq; pruned.witnesses_missing, 0);
    check!(store.receipt("old-operation")?.is_none());
    check!(eq; store.receipt_count()?, 0);
    let reapplied = apply(&store, "old-operation", "digest-old", "unit-old")?;
    check!(
        matches!(reapplied, Application::Written(_)),
        "{reapplied:?}"
    );
    check!(eq; rows(&store, Table::Schools)?, 4);
    Ok(())
}

#[test]
fn a_boundary_that_is_not_a_day_is_refused() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    apply(
        &store,
        "wiaa_schools_wi:w39:schools:0",
        "digest-a",
        "unit-0",
    )?;
    let refused = store.prune_receipts(&frontier("last tuesday"));
    check!(
        matches!(refused, Err(StoreError::Refused { .. })),
        "an unreadable boundary is refused rather than guessed at: {refused:?}"
    );
    check!(eq; store.receipt_count()?, 1, "a refused prune removed nothing");
    check!(
        store.receipt_frontier()?.is_none(),
        "a refused prune advances no frontier"
    );
    Ok(())
}

#[test]
fn a_witness_retains_its_receipt_past_the_day_boundary() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let tomorrow = day(1)?;
    apply(
        &store,
        "wiaa_schools_wi:w39:schools:0",
        "digest-a",
        "unit-0",
    )?;
    apply(
        &store,
        "wiaa_schools_wi:w39:schools:1",
        "digest-b",
        "unit-1",
    )?;
    let pruned = store.prune_receipts(&ReplayFrontier {
        at: tomorrow,
        witnesses: vec![witness("invocation-7", "wiaa_schools_wi:w39:schools:1")],
    })?;
    check!(eq; pruned.removed, 1, "the receipt no active invocation names is past the boundary");
    check!(eq; pruned.witnesses_kept, 1);
    check!(eq; pruned.witnesses_missing, 0);
    check!(
        store.receipt("wiaa_schools_wi:w39:schools:1")?.is_some(),
        "the witness an active invocation names is retained"
    );
    check!(store.receipt("wiaa_schools_wi:w39:schools:0")?.is_none());
    check!(eq; store.receipt_count()?, 1);
    Ok(())
}

#[test]
fn a_witness_with_no_standing_receipt_is_reported() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let yesterday = day(-1)?;
    apply(
        &store,
        "wiaa_schools_wi:w39:schools:0",
        "digest-a",
        "unit-0",
    )?;
    let pruned = store.prune_receipts(&ReplayFrontier {
        at: yesterday,
        witnesses: vec![
            witness("invocation-7", "wiaa_schools_wi:w39:schools:0"),
            witness("invocation-8", "never-applied"),
        ],
    })?;
    check!(eq; pruned.removed, 0);
    check!(eq; pruned.witnesses_kept, 1);
    check!(
        eq;
        pruned.witnesses_missing,
        1,
        "an invocation whose witness is gone is reported, not assumed recoverable"
    );
    check!(store.receipt("wiaa_schools_wi:w39:schools:0")?.is_some());
    Ok(())
}

#[test]
fn a_frontier_that_moves_backwards_is_refused() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let yesterday = day(-1)?;
    let tomorrow = day(1)?;
    apply(&store, "old-operation", "digest-old", "unit-old")?;
    store.prune_receipts(&frontier(&tomorrow))?;
    let refused = store.prune_receipts(&frontier(&yesterday));
    check!(
        matches!(refused, Err(StoreError::Refused { .. })),
        "a frontier behind the standing one re-opens a window this store already pruned: {refused:?}"
    );
    check!(eq; store.receipt_frontier()?.ok_or("standing frontier")?.at, tomorrow);
    let same = store.prune_receipts(&frontier(&tomorrow))?;
    check!(
        !same.frontier_advanced,
        "pruning again against the standing day advances nothing"
    );
    check!(eq; same.removed, 0);
    Ok(())
}

#[test]
fn the_consumed_frontier_survives_reopening_with_its_witnesses() -> TestResult {
    let dir = tempfile::tempdir()?;
    let tomorrow = day(1)?;
    {
        let store = Store::open(dir.path())?;
        apply(&store, "old-operation", "digest-old", "unit-old")?;
        let pruned = store.prune_receipts(&ReplayFrontier {
            at: tomorrow.clone(),
            witnesses: vec![witness("invocation-7", "old-operation")],
        })?;
        check!(eq; pruned.removed, 0);
        check!(eq; pruned.witnesses_kept, 1);
    }
    let store = Store::open(dir.path())?;
    let standing = store.receipt_frontier()?.ok_or("standing frontier")?;
    check!(eq; standing.at, tomorrow);
    check!(eq; standing.witnesses.len(), 1, "the witness set is part of the durable frontier");
    check!(eq; standing.witnesses.first().map(|witness| witness.operation.as_str()), Some("old-operation"));
    let refused = store.prune_receipts(&frontier(&day(-1)?));
    check!(
        matches!(refused, Err(StoreError::Refused { .. })),
        "the standing frontier survives the reopen, so a day behind it is still refused: {refused:?}"
    );
    Ok(())
}

#[test]
fn a_witness_the_store_cannot_record_is_refused_before_pruning() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let tomorrow = day(1)?;
    apply(
        &store,
        "wiaa_schools_wi:w39:schools:0",
        "digest-a",
        "unit-0",
    )?;
    let long = "o".repeat(MAX_OPERATION_BYTES + 1);
    let cases = [
        (
            witness("", "wiaa_schools_wi:w39:schools:0"),
            "a witness naming no invocation",
        ),
        (witness("invocation-7", ""), "a witness naming no operation"),
        (
            witness(long.as_str(), "wiaa_schools_wi:w39:schools:0"),
            "an invocation past the id ceiling",
        ),
        (
            witness("invocation-7", long.as_str()),
            "an operation past the id ceiling",
        ),
    ];
    for (case, why) in cases {
        let refused = store.prune_receipts(&ReplayFrontier {
            at: tomorrow.clone(),
            witnesses: vec![case],
        });
        check!(
            matches!(refused, Err(StoreError::Refused { .. })),
            "{why} is refused before anything is pruned: {refused:?}"
        );
    }
    check!(eq; store.receipt_count()?, 1, "a refused prune removed nothing");
    let too_many: Vec<ReplayWitness> = (0..=MAX_FRONTIER_WITNESSES)
        .map(|index| witness("invocation-7", &format!("op-{index}")))
        .collect();
    let refused = store.prune_receipts(&ReplayFrontier {
        at: tomorrow,
        witnesses: too_many,
    });
    check!(
        matches!(refused, Err(StoreError::Refused { .. })),
        "a witness set past the ceiling is refused, so one prune cannot carry unbounded input: {refused:?}"
    );
    check!(eq; store.receipt_count()?, 1);
    Ok(())
}

#[test]
fn an_id_or_digest_the_store_cannot_record_is_refused() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let long_id = "o".repeat(MAX_OPERATION_BYTES + 1);
    let long_digest = "d".repeat(MAX_DIGEST_BYTES + 1);
    let cases = [
        ("", "digest-a", "an operation keyed by nothing"),
        ("operation", "", "a digest that matches every payload"),
        (long_id.as_str(), "digest-a", "an id past the key ceiling"),
        (
            "operation",
            long_digest.as_str(),
            "a digest past the value ceiling",
        ),
    ];
    for (operation, digest, why) in cases {
        let mut batch = store.write_batch();
        batch.append_many(Table::Schools, &page())?;
        let refused = batch.commit_once(operation, digest);
        check!(
            matches!(refused, Err(StoreError::Refused { .. })),
            "{why} is refused before anything is written: {refused:?}"
        );
    }
    check!(eq; rows(&store, Table::Schools)?, 0);
    check!(eq; store.receipt_count()?, 0);
    Ok(())
}

#[test]
fn a_credited_operation_counts_its_rows_once_however_often_it_is_offered() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let operation = "wiaa_schools_wi:w39:schools:0";
    let first = apply(&store, operation, "digest-a", "unit-0")?;
    check!(first.written(), "{first:?}");
    let credit = store.credit_observations("wiaa_schools", operation)?;
    check!(eq; credit.credited, 2, "the first credit counts the applied rows");
    check!(eq; credit.total, 2, "and the endpoint total starts there");
    let replay = apply(&store, operation, "digest-a", "unit-0")?;
    check!(replay.repeated(), "{replay:?}");
    let again = store.credit_observations("wiaa_schools", operation)?;
    check!(eq; again.credited, 0, "a repeated credit adds nothing");
    check!(eq; again.total, 2, "and leaves the total standing");
    check!(eq; store.endpoint_credit("wiaa_schools")?, 2);
    check!(eq; store.endpoint_credit("milesplit_wi")?, 0, "totals are per endpoint");
    Ok(())
}

#[test]
fn an_endpoints_total_survives_reopening_and_accumulates_across_operations() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    apply(&store, "op-0", "digest-0", "unit-0")?;
    apply(&store, "op-1", "digest-1", "unit-1")?;
    check!(eq; store.credit_observations("wiaa_schools", "op-0")?.total, 2);
    check!(eq; store.credit_observations("wiaa_schools", "op-1")?.total, 4);
    check!(eq; store.credit_observations("wiaa_schools", "op-0")?.credited, 0);
    drop(store);
    let reopened = Store::open(dir.path())?;
    check!(eq; reopened.endpoint_credit("wiaa_schools")?, 4, "the total is durable");
    check!(eq; reopened.credit_observations("wiaa_schools", "op-1")?.credited, 0);
    check!(eq; reopened.credit_observations("wiaa_schools", "op-1")?.total, 4);
    Ok(())
}

#[test]
fn an_operation_without_a_receipt_is_never_credited() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let refused = store.credit_observations("wiaa_schools", "never_applied");
    check!(
        matches!(refused, Err(StoreError::Refused { .. })),
        "an unapplied operation is refused rather than counted: {refused:?}"
    );
    let unnamed = store.credit_observations("", "never_applied");
    check!(
        matches!(unnamed, Err(StoreError::Refused { .. })),
        "an empty endpoint is refused rather than totalled into nothing: {unnamed:?}"
    );
    check!(eq; store.endpoint_credit("wiaa_schools")?, 0);
    Ok(())
}

#[test]
fn an_incomplete_replay_frontier_cannot_delete_other_recovery_receipts() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    apply(&store, "lost-ack", "digest-old", "unit-old")?;
    let pruned = store.prune_receipts(&ReplayFrontier {
        at: day(1)?,
        witnesses: vec![witness("active-invocation", "missing-recovery-witness")],
    })?;
    check!(eq; pruned.witnesses_missing, 1);
    check!(eq; pruned.removed, 0, "an incomplete frontier must retain every receipt");
    check!(store.receipt("lost-ack")?.is_some());
    check!(store.receipt_frontier()?.is_none());
    Ok(())
}
