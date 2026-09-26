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
    row.evidence
        .push(Evidence::parsed(SourceRef::id(source), at));
    row
}

/// A table's mark as the store holds it, read the way an operator reads `meta`.
fn mark(store: &Store, table: Table) -> Option<u64> {
    let value = store.meta.get(mark_key(table)).unwrap()?;
    Some(std::str::from_utf8(&value).unwrap().trim().parse().unwrap())
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
    let dir = tempfile::tempdir().unwrap();
    {
        let store = Store::open(dir.path()).unwrap();
        store
            .append(
                Table::Schools,
                &observed("Abbotsford", "wiaa_schools", "2026-09-19"),
            )
            .unwrap();
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

    let store = Store::open(dir.path()).unwrap();
    assert_eq!(mark(&store, Table::Schools), Some(2));
    assert_eq!(counter(&store, Table::Schools), 2);
}

#[test]
fn an_import_carries_the_row_count_it_resumed_from() {
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
    store.meta.remove("imported:schools").unwrap();
    store.import_legacy().unwrap();
    assert_eq!(
        store.count(Table::Schools).unwrap(),
        1,
        "the import counts the row it stored"
    );

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
    let dir = tempfile::tempdir().unwrap();
    {
        let store = Store::open(dir.path()).unwrap();
        store
            .append(
                Table::Schools,
                &observed("Abbotsford", "wiaa_schools", "2026-09-19"),
            )
            .unwrap();
        store
            .entities
            .insert(b"schools\0broken".as_slice(), b"{}".as_slice())
            .unwrap();
    }
    let store = Store::open(dir.path()).unwrap();
    assert_eq!(counter(&store, Table::Schools), 1);

    store.meta.remove(mark_key(Table::Schools)).unwrap();
    store.flush().unwrap();
    drop(store);
    match Store::open(dir.path()).map(|_| ()) {
        Err(StoreError::Invariant { detail }) => {
            assert!(
                detail.contains("malformed observation key"),
                "unexpected detail: {detail}"
            )
        }
        other => panic!("expected the walk to refuse the unparseable key, got {other:?}"),
    }
}

#[test]
fn concurrent_appends_leave_a_mark_a_reopen_resumes_from() {
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
