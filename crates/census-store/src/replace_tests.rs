//! Tests for a batch that derives: several tables' whole content, staged and committed as one.
//!
//! A review pass answers a set of cases and must write what it answered and the cases' new state as
//! one thing. Written as two single-table replacements — each atomic on its own — a crash between
//! them leaves a verdict standing against a case that still reads as open. These tests hold the batch
//! to that: both tables reach the database together, a replay of the same operation changes neither,
//! and a batch that never commits leaves both as they were.

use super::*;
use serde::{Deserialize, Serialize};

/// A derived table's row, as the store's own staging sees it: an `id`, and something to read back.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Row {
    id: String,
    note: String,
}

impl Entity for Row {
    fn entity_id(&self) -> &str {
        &self.id
    }

    fn merge(&mut self, other: Self) {
        *self = other;
    }
}

fn row(id: &str, note: &str) -> Row {
    Row {
        id: id.to_string(),
        note: note.to_string(),
    }
}

fn rows_of(store: &Store, table: Table) -> Vec<Row> {
    let mut rows = store.scan::<Row>(table).unwrap();
    rows.sort_by(|left, right| left.id.cmp(&right.id));
    rows
}

fn count(store: &Store, table: Table) -> u64 {
    store
        .stats()
        .unwrap()
        .tables
        .into_iter()
        .find(|(name, _)| name == table.file())
        .map(|(_, count)| count)
        .unwrap()
}

/// One review pass: the verdicts it minted and the cases it closed, in the caller's commit.
fn derive<'s>(store: &'s Store, verdicts: &[Row], cases: &[Row]) -> StoreBatch<'s> {
    let mut batch = store.write_batch();
    batch
        .replace_many(Table::IdentityVerdicts, verdicts)
        .unwrap();
    batch.replace_many(Table::ReviewCases, cases).unwrap();
    batch
}

#[test]
fn one_derivation_writes_its_tables_in_one_commit() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let verdicts = vec![row("verdict-1", "decided"), row("verdict-2", "refused")];
    let cases = vec![row("case-1", "closed"), row("case-2", "retained")];

    let applied = derive(&store, &verdicts, &cases)
        .commit_once("review:2026-09-25:7", "digest-7")
        .unwrap();

    assert!(
        applied.written(),
        "the first application wrote the derivation"
    );
    assert_eq!(
        rows_of(&store, Table::IdentityVerdicts),
        verdicts,
        "the verdicts are readable"
    );
    assert_eq!(
        rows_of(&store, Table::ReviewCases),
        cases,
        "and the cases' new state is readable beside them"
    );
    assert_eq!(count(&store, Table::IdentityVerdicts), 2);
    assert_eq!(count(&store, Table::ReviewCases), 2);
    assert_eq!(
        store.receipt_count().unwrap(),
        1,
        "one derivation, one receipt"
    );
}

#[test]
fn a_replayed_derivation_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let verdicts = vec![row("verdict-1", "decided")];
    let cases = vec![row("case-1", "closed")];

    derive(&store, &verdicts, &cases)
        .commit_once("review:2026-09-25:7", "digest-7")
        .unwrap();
    let replay = derive(&store, &verdicts, &cases)
        .commit_once("review:2026-09-25:7", "digest-7")
        .unwrap();

    assert!(
        replay.repeated(),
        "the second application is the first's replay"
    );
    assert_eq!(rows_of(&store, Table::IdentityVerdicts), verdicts);
    assert_eq!(rows_of(&store, Table::ReviewCases), cases);
    assert_eq!(count(&store, Table::IdentityVerdicts), 1);
    assert_eq!(count(&store, Table::ReviewCases), 1);
    assert_eq!(store.receipt_count().unwrap(), 1);
}

#[test]
fn a_derivation_that_never_commits_leaves_both_tables_as_they_were() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    store
        .replace_many(Table::ReviewCases, &[row("case-1", "open")])
        .unwrap();

    drop(derive(
        &store,
        &[row("verdict-1", "decided")],
        &[row("case-1", "closed")],
    ));

    assert!(
        rows_of(&store, Table::IdentityVerdicts).is_empty(),
        "the dropped batch left no verdict"
    );
    assert_eq!(
        rows_of(&store, Table::ReviewCases),
        vec![row("case-1", "open")],
        "and the case still reads as the last commit left it"
    );
}

#[test]
fn a_snapshot_table_replaced_by_nothing_comes_out_empty() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let held = vec![row("wi", "covered"), row("mn", "covered")];
    {
        let mut batch = store.write_batch();
        batch.replace_many(Table::Coverage, &held).unwrap();
        batch.commit().unwrap();
    }
    assert_eq!(count(&store, Table::Coverage), 2);

    let mut empty = store.write_batch();
    empty
        .replace_many(Table::Coverage, &Vec::<Row>::new())
        .unwrap();
    empty.commit().unwrap();

    assert!(
        rows_of(&store, Table::Coverage).is_empty(),
        "a snapshot derivation names the table's whole content, so naming none empties it"
    );
    assert_eq!(count(&store, Table::Coverage), 0);
}

#[test]
fn one_table_cannot_be_appended_and_replaced_in_one_batch() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let mut batch = store.write_batch();
    batch
        .append_many(Table::Schools, &[row("school-1", "appended")])
        .unwrap();

    let refused = batch.replace_many(Table::Schools, &[row("school-1", "replaced")]);
    assert!(
        refused.is_err(),
        "an append and a replacement disagree about what the table holds"
    );
    drop(batch);

    assert!(
        store.scan::<Row>(Table::Schools).unwrap().is_empty(),
        "and a refused call writes nothing"
    );
}
