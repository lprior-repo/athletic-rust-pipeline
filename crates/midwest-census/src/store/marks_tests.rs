//! Tests for the durable per-table sequence marks: the `meta` rows that let an open resume every
//! table without walking one, and the one scan a store written before them pays.

use std::sync::Arc;

use super::sequences::mark_key;
use super::*;
use census_domain::model::*;
use census_domain::UsJurisdiction;

fn school(name: &str) -> CanonicalSchool {
    CanonicalSchool::new(UsJurisdiction::Wisconsin, name, normalize_name(name)).0
}

/// One observation of `name` carrying `source` as its evidence, so an entity that was observed twice
/// shows both observations once they are merged by a scan.
fn observed(name: &str, source: &str, at: &str) -> CanonicalSchool {
    let mut row = school(name);
    row.evidence.push(Evidence::parsed(SourceRef::id(source), at));
    row
}

/// A table's mark as the store holds it, read the way an operator reads `meta`.
fn mark(store: &Store, table: Table) -> Option<u64> {
    let value = store.meta.get(mark_key(table)).unwrap()?;
    Some(
        std::str::from_utf8(&value)
            .unwrap()
            .trim()
            .parse()
            .unwrap(),
    )
}

/// The sequence a table's next append will use, as [`Store::stats`] reports it: the pointer a reopen
/// seeds from, which is what a mark is.
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

#[test]
fn a_batch_stores_the_mark_its_sequences_land_in() {
    // The mark is the durable half of the counter: the sequence the table's next append will use, and
    // so the number a reopen seeds that table from. It is stored *inside* the batch that spends the
    // sequences, so the two cannot disagree — a mark written before its batch would outlive a batch
    // that never committed.
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    assert_eq!(
        mark(&store, Table::Schools),
        Some(0),
        "an open leaves every table with a mark"
    );

    store
        .append_many(
            Table::Schools,
            &[
                observed("Abbotsford", "wiaa_schools", "2026-09-19"),
                observed("Colby", "wiaa_schools", "2026-09-19"),
            ],
        )
        .unwrap();
    assert_eq!(mark(&store, Table::Schools), Some(2));
    assert_eq!(counter(&store, Table::Schools), 2);

    // A refused batch reserves nothing, so it may not move the mark either: an id past the key
    // contract's ceiling is refused before a single sequence is handed out.
    let oversized = serde_json::json!({ "id": "s".repeat(MAX_ID_BYTES + 1) });
    assert!(store.append(Table::Schools, &oversized).is_err());
    assert_eq!(
        mark(&store, Table::Schools),
        Some(2),
        "a refused batch may not move the mark"
    );
    assert_eq!(counter(&store, Table::Schools), 2);
}

#[test]
fn a_reopened_store_resumes_at_the_mark_its_last_batch_committed() {
    // The round trip: the sequence the counter sat at before the store was closed is the sequence the
    // next open resumes at, and the observation written after the reopen lands beside its entity's
    // first one instead of over it.
    let dir = tempfile::tempdir().unwrap();
    {
        let store = Store::open(dir.path()).unwrap();
        store
            .append(
                Table::Schools,
                &observed("Abbotsford", "wiaa_schools", "2026-09-19"),
            )
            .unwrap();
        assert_eq!(mark(&store, Table::Schools), Some(1));
    }
    let store = Store::open(dir.path()).unwrap();
    assert_eq!(
        mark(&store, Table::Schools),
        Some(1),
        "the mark survives the reopen"
    );
    assert_eq!(
        counter(&store, Table::Schools),
        1,
        "the same next sequence the last batch left"
    );

    let mut again = observed("Abbotsford", "mshsl_schools", "2026-09-20");
    again.city = Some("Abbotsford".into());
    store.append(Table::Schools, &again).unwrap();
    assert_eq!(mark(&store, Table::Schools), Some(2));
    let rows = store.scan::<CanonicalSchool>(Table::Schools).unwrap();
    let resumed = rows.iter().find(|row| row.id == again.id).unwrap();
    assert_eq!(
        resumed.evidence.len(),
        2,
        "the resumed append must not overwrite the first observation"
    );
    assert_eq!(resumed.city.as_deref(), Some("Abbotsford"));
}

#[test]
fn a_store_written_without_marks_learns_them_from_one_scan() {
    // Every store written before marks existed holds none, and its observations are the only record of
    // where its tables resume. The open that misses a mark derives it with the one scan this store
    // always did and commits the result, so the table is walked once and never again — and the number
    // it derives is the number the scan-derived counter held before, so which sequence a later append
    // takes does not change.
    let dir = tempfile::tempdir().unwrap();
    {
        let store = Store::open(dir.path()).unwrap();
        store
            .append(
                Table::Schools,
                &observed("Abbotsford", "wiaa_schools", "2026-09-19"),
            )
            .unwrap();
        // What the older code path leaves behind: rows and sequences, no mark — and an empty table
        // loses its mark too, because the write-back has to cover every table a scan established.
        store.meta.remove(mark_key(Table::Schools)).unwrap();
        store.meta.remove(mark_key(Table::Athletes)).unwrap();
        store.flush().unwrap();
    }
    {
        let store = Store::open(dir.path()).unwrap();
        assert_eq!(
            mark(&store, Table::Schools),
            Some(1),
            "the open derived the mark it was missing"
        );
        assert_eq!(
            mark(&store, Table::Athletes),
            Some(0),
            "an empty table is marked zero"
        );
        assert_eq!(counter(&store, Table::Schools), 1);

        let mut again = observed("Abbotsford", "mshsl_schools", "2026-09-20");
        again.city = Some("Abbotsford".into());
        store.append(Table::Schools, &again).unwrap();
        assert_eq!(mark(&store, Table::Schools), Some(2));
        let rows = store.scan::<CanonicalSchool>(Table::Schools).unwrap();
        let resumed = rows.iter().find(|row| row.id == again.id).unwrap();
        assert_eq!(
            resumed.evidence.len(),
            2,
            "the migration resumes at the sequence the scan found"
        );
    }

    // The scan was a migration and not a habit: the open after it reads the mark it wrote.
    let store = Store::open(dir.path()).unwrap();
    assert_eq!(mark(&store, Table::Schools), Some(2));
    assert_eq!(counter(&store, Table::Schools), 2);
}

#[test]
fn an_import_carries_the_row_count_it_resumed_from() {
    // The legacy import writes rows after the open that seeded each table's count, so the count has to
    // ride the import's own batches. A batch that wrote only rows would leave the table holding more
    // rows than it reports, which `integrity` reads as a mismatch; the count a resumed import carries
    // is the table's total, not the chunk's.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    std::fs::create_dir_all(root.join("entities")).unwrap();
    let store = Store::open(&root).unwrap();
    assert_eq!(
        store.count(Table::Schools).unwrap(),
        0,
        "an open seeds the count of an empty table"
    );

    let mut journal = serde_json::to_string(&school("Abbotsford")).unwrap();
    journal.push('\n');
    std::fs::write(root.join("entities/schools.jsonl"), &journal).unwrap();
    // The open already walked every table and recorded this one as imported (its journal did not exist
    // yet), so a file that appears afterwards is out of scope until its marker is dropped - the same
    // reason the interrupted-import fixture drops one.
    store.meta.remove("imported:schools").unwrap();
    store.import_legacy().unwrap();
    assert_eq!(
        store.count(Table::Schools).unwrap(),
        1,
        "the import counts the row it stored"
    );

    // The next run meets a journal that grew by one line and resumes at the offset the first run
    // committed, so this chunk is one row against a table that already holds one.
    journal.push_str(&serde_json::to_string(&school("Colby")).unwrap());
    journal.push('\n');
    std::fs::write(root.join("entities/schools.jsonl"), &journal).unwrap();
    store.meta.remove("imported:schools").unwrap();
    store.import_legacy().unwrap();
    assert_eq!(store.count(Table::Schools).unwrap(), 2);
    assert_eq!(
        store.scan::<CanonicalSchool>(Table::Schools).unwrap().len(),
        2
    );
}

#[test]
fn an_open_with_a_mark_never_walks_the_table() {
    // `Store::open` is O(tables) because each table's resume point is a `meta` row. The walk a missing
    // mark still needs refuses a key it cannot parse — `last_sequence` returns `Invariant` — so a
    // table holding one is a table an open that walked it could not open at all. The store holds a
    // mark for every table, so this store opens; dropping the mark is what makes the same store fail,
    // which is what pins the walk to the migration path and nothing else.
    let dir = tempfile::tempdir().unwrap();
    {
        let store = Store::open(dir.path()).unwrap();
        store
            .append(
                Table::Schools,
                &observed("Abbotsford", "wiaa_schools", "2026-09-19"),
            )
            .unwrap();
        // A key under `schools\0` that is not `<id>\0<sequence>`: the one row the store's own writer
        // can never produce, and the row a walk of that table trips over.
        store
            .entities
            .insert(b"schools\0broken".as_slice(), b"{}".as_slice())
            .unwrap();
    }
    let store = Store::open(dir.path()).unwrap();
    assert_eq!(counter(&store, Table::Schools), 1);

    // A mark that cannot be established fails the open with a typed error instead of guessing: the
    // number a guess would resume from is one the store may already hold.
    store.meta.remove(mark_key(Table::Schools)).unwrap();
    store.flush().unwrap();
    drop(store);
    match Store::open(dir.path()).map(|_| ()) {
        Err(StoreError::Invariant { detail }) => {
            assert!(detail.contains("malformed observation key"), "unexpected detail: {detail}")
        }
        other => panic!("expected the walk to refuse the unparseable key, got {other:?}"),
    }
}

#[test]
fn concurrent_appends_leave_a_mark_a_reopen_resumes_from() {
    // Writers reserve under the append lock and commit under it, so the batches that spend one table's
    // sequences commit in the order they reserved, and the last commit leaves that table's high-water
    // mark. This is a stress test — it covers the interleaving rather than proving it — and it is the
    // shape production produces, where several jobs share a table.
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(Store::open(dir.path()).unwrap());
    let writers = 4_usize;
    let per_writer = 25_usize;
    let mut threads = Vec::with_capacity(writers);
    for writer in 0..writers {
        let store = Arc::clone(&store);
        threads.push(std::thread::spawn(move || {
            for index in 0..per_writer {
                let row = school(&format!("School {writer} {index}"));
                store.append(Table::Schools, &row).unwrap();
            }
        }));
    }
    for thread in threads {
        thread.join().unwrap();
    }
    let appended = u64::try_from(writers * per_writer).unwrap();
    assert_eq!(mark(&store, Table::Schools), Some(appended));
    drop(store);

    let store = Store::open(dir.path()).unwrap();
    assert_eq!(
        mark(&store, Table::Schools),
        Some(appended),
        "the last commit left the table's high-water mark"
    );
    assert_eq!(counter(&store, Table::Schools), appended);
    assert_eq!(
        store.scan::<CanonicalSchool>(Table::Schools).unwrap().len(),
        usize::try_from(appended).unwrap(),
        "no append may overwrite another writer's observation"
    );
}
