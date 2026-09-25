//! Tests for receipted commits: one operation, one commit, one receipt — and what a replay of that
//! operation does to the rows, the counters and the journal beside it.

use super::*;
use census_domain::model::*;
use census_domain::UsJurisdiction;
use chrono::NaiveDate;

use super::clock::{Clock, SystemClock};

fn school(name: &str) -> CanonicalSchool {
    CanonicalSchool::new(UsJurisdiction::Wisconsin, name, normalize_name(name)).0
}

/// One observation of `name` carrying `source` as its evidence.
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

/// Rows a table holds, as [`Store::stats`] reports them.
fn rows(store: &Store, table: Table) -> u64 {
    store
        .stats()
        .unwrap()
        .tables
        .into_iter()
        .find(|(name, _)| name == table.file())
        .map(|(_, count)| count)
        .unwrap()
}

/// The sequence a table's next append will use: the pointer a replay must not move.
fn counter(store: &Store, table: Table) -> u64 {
    store
        .stats()
        .unwrap()
        .appended
        .into_iter()
        .find(|(name, _)| name == table.file())
        .map(|(_, next)| next)
        .unwrap()
}

/// Apply one operation through one batch: its page, its journal entry and its receipt commit
/// together, exactly as the ingest path applies a posted page.
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

#[test]
fn a_replay_of_one_operation_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let first = apply(
        &store,
        "wiaa_schools_wi:w39:schools:0",
        "digest-a",
        "unit-0",
    )
    .unwrap();
    let Application::Written(receipt) = &first else {
        panic!("the first application of an operation writes its receipt: {first:?}");
    };
    assert_eq!(receipt.appended, 2);
    assert_eq!(receipt.operation, "wiaa_schools_wi:w39:schools:0");
    assert_eq!(rows(&store, Table::Schools), 2);

    // The replay the mechanism exists for: the same operation, the same payload digest, arriving
    // again because the acknowledgement of the first application was lost.
    let repeat = apply(
        &store,
        "wiaa_schools_wi:w39:schools:0",
        "digest-a",
        "unit-0",
    )
    .unwrap();
    let Application::Repeated(standing) = &repeat else {
        panic!("a repeat of an applied operation is not written again: {repeat:?}");
    };
    assert_eq!(
        repeat.appended(),
        0,
        "a replay appends nothing, so a caller summing replies cannot count the page twice"
    );
    assert_eq!(
        standing, receipt,
        "the replay answers with the receipt the first application left"
    );
    assert_eq!(rows(&store, Table::Schools), 2, "no row was written twice");
    assert_eq!(
        store.receipt_count().unwrap(),
        1,
        "one operation, one receipt — the replay did not add another"
    );
}

#[test]
fn one_operation_id_cannot_name_two_payloads() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    apply(
        &store,
        "wiaa_schools_wi:w39:schools:0",
        "digest-a",
        "unit-0",
    )
    .unwrap();

    // The same id, a different payload: a caller that reused a name for different work. Appending
    // it would put rows under an id whose receipt describes other rows, so the commit is refused
    // and the store keeps what it holds.
    let refused = apply(
        &store,
        "wiaa_schools_wi:w39:schools:0",
        "digest-b",
        "unit-0",
    );
    let Err(error) = refused else {
        panic!("a changed payload under an applied id is refused: {refused:?}");
    };
    assert!(
        matches!(error, StoreError::Invariant { .. }),
        "the refusal is an invariant violation, which is terminal: {error:?}"
    );
    let message = error.to_string();
    assert!(
        message.contains("wiaa_schools_wi:w39:schools:0"),
        "the refusal names the operation: {message}"
    );
    assert!(
        message.contains("digest-a") && message.contains("digest-b"),
        "the refusal names both digests: {message}"
    );

    assert_eq!(
        rows(&store, Table::Schools),
        2,
        "the refused page wrote no rows"
    );
    assert_eq!(
        store.receipt_count().unwrap(),
        1,
        "the refused page wrote no receipt"
    );
    let receipt = store
        .receipt("wiaa_schools_wi:w39:schools:0")
        .unwrap()
        .unwrap();
    assert_eq!(
        receipt.digest, "digest-a",
        "the standing receipt is the first one"
    );
}

#[test]
fn the_receipt_outlives_the_process_that_wrote_it() {
    let dir = tempfile::tempdir().unwrap();
    let operation = "wiaa_schools_wi:w39:schools:0";
    {
        let store = Store::open(dir.path()).unwrap();
        apply(&store, operation, "digest-a", "unit-0").unwrap();
        // No flush, no close: the commit is the durability boundary, so what a crash leaves is what
        // this leaves.
    }

    // A writer that died after the append and before it heard its own acknowledgement comes back
    // to a store that already holds the page, and must be told so.
    let store = Store::open(dir.path()).unwrap();
    let receipt = store.receipt(operation).unwrap().unwrap();
    assert_eq!(receipt.appended, 2);
    let repeat = apply(&store, operation, "digest-a", "unit-0").unwrap();
    assert!(matches!(repeat, Application::Repeated(_)), "{repeat:?}");
    assert_eq!(rows(&store, Table::Schools), 2);
}

#[test]
fn a_replay_moves_no_counter_and_writes_no_second_journal_entry() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    apply(
        &store,
        "wiaa_schools_wi:w39:schools:0",
        "digest-a",
        "unit-0",
    )
    .unwrap();
    let rows_before = rows(&store, Table::Schools);
    let counter_before = counter(&store, Table::Schools);
    let journal = store.journal_keys("wiaa_schools").unwrap();
    assert!(
        journal.contains("unit-0"),
        "the operation's marker was written"
    );

    let repeat = apply(
        &store,
        "wiaa_schools_wi:w39:schools:0",
        "digest-a",
        "unit-0",
    )
    .unwrap();
    assert!(matches!(repeat, Application::Repeated(_)), "{repeat:?}");

    // A repeat that reserved sequences would leave the table's pointer ahead of the rows the store
    // holds, and a reopen would hand out those sequences again.
    assert_eq!(counter(&store, Table::Schools), counter_before);
    assert_eq!(rows(&store, Table::Schools), rows_before);
    assert_eq!(
        store.journal_keys("wiaa_schools").unwrap(),
        journal,
        "the marker a replay would rewrite is the one already standing"
    );
}

#[test]
fn an_operation_with_no_rows_still_has_a_receipt() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let mut batch = store.write_batch();
    batch
        .append_many(Table::Schools, &Vec::<CanonicalSchool>::new())
        .unwrap();
    assert!(batch.is_empty(), "an empty page buffers nothing");

    let outcome = batch
        .commit_once("wiaa_schools_wi:w39:schools:empty", "digest-e")
        .unwrap();
    assert_eq!(
        outcome.appended(),
        0,
        "an operation with no rows appends none"
    );
    assert!(
        matches!(outcome, Application::Written(_)),
        "and is still applied"
    );
    assert_eq!(store.receipt_count().unwrap(), 1);

    let mut batch = store.write_batch();
    batch.append_many(Table::Schools, &page()).unwrap();
    let repeat = batch
        .commit_once("wiaa_schools_wi:w39:schools:empty", "digest-e")
        .unwrap();
    let Application::Repeated(receipt) = repeat else {
        panic!("the empty operation is a repeat, so its page is not written: {repeat:?}");
    };
    assert_eq!(receipt.digest, "digest-e");
    assert_eq!(rows(&store, Table::Schools), 0, "the repeat wrote no rows");
}

#[test]
fn receipts_older_than_the_policy_day_are_removed_and_newer_ones_kept() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let today = SystemClock.today();
    let yesterday =
        (NaiveDate::parse_from_str(&today, "%Y-%m-%d").unwrap() - chrono::Days::new(1)).to_string();
    apply(&store, "old-operation", "digest-old", "unit-old").unwrap();

    // A boundary that has not reached the receipt's day removes nothing: the window it guards is
    // still open, and deleting it would re-open the double append it exists to prevent.
    let kept = store.prune_receipts(&yesterday).unwrap();
    assert_eq!(kept, Pruned::default(), "nothing older than yesterday");
    assert!(store.receipt("old-operation").unwrap().is_some());

    let tomorrow =
        (NaiveDate::parse_from_str(&today, "%Y-%m-%d").unwrap() + chrono::Days::new(1)).to_string();
    let pruned = store.prune_receipts(&tomorrow).unwrap();
    assert_eq!(pruned.removed, 1);
    assert_eq!(pruned.undated, 0);
    assert!(store.receipt("old-operation").unwrap().is_none());
    assert_eq!(store.receipt_count().unwrap(), 0);

    // With the receipt gone the operation is free to be applied again, which is the point of a
    // retention policy: past the window a replay cannot arrive, and growth is bounded.
    let reapplied = apply(&store, "old-operation", "digest-old", "unit-old").unwrap();
    assert!(
        matches!(reapplied, Application::Written(_)),
        "{reapplied:?}"
    );
    assert_eq!(rows(&store, Table::Schools), 4);
}

#[test]
fn a_boundary_that_is_not_a_day_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    apply(
        &store,
        "wiaa_schools_wi:w39:schools:0",
        "digest-a",
        "unit-0",
    )
    .unwrap();
    let refused = store.prune_receipts("last tuesday");
    assert!(
        matches!(refused, Err(StoreError::Refused { .. })),
        "an unreadable boundary is refused rather than guessed at: {refused:?}"
    );
    assert_eq!(
        store.receipt_count().unwrap(),
        1,
        "a refused prune removed nothing"
    );
}

#[test]
fn an_id_or_digest_the_store_cannot_record_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
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
        batch.append_many(Table::Schools, &page()).unwrap();
        let refused = batch.commit_once(operation, digest);
        assert!(
            matches!(refused, Err(StoreError::Refused { .. })),
            "{why} is refused before anything is written: {refused:?}"
        );
    }
    assert_eq!(rows(&store, Table::Schools), 0);
    assert_eq!(store.receipt_count().unwrap(), 0);
}
