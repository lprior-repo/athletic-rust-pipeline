use super::*;

enum Attempt<'a> {
    Record(&'a CanonicalSchool),
    Fail,
}

impl serde::Serialize for Attempt<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Record(record) => record.serialize(serializer),
            Self::Fail => Err(serde::ser::Error::custom("injected encode failure")),
        }
    }
}

#[test]
fn a_store_batch_encode_failure_commits_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let kept = school("Kept");
    store.append(Table::Schools, &kept).unwrap();
    let before_seq = sequence_pointer(&store, Table::Schools);
    let before_rows = rows_held(&store, Table::Schools);
    let mut batch = store.write_batch();
    batch
        .journal_done("unit", "fail-atomic", &serde_json::json!({"rows": 3}))
        .unwrap();
    let first = school("First Staged");
    let second = school("Second Staged");
    let refused = batch.append_many(
        Table::Schools,
        &[
            Attempt::Record(&first),
            Attempt::Record(&second),
            Attempt::Fail,
        ],
    );
    assert!(matches!(refused, Err(StoreError::Json { .. })));
    drop(batch);
    drop(store);
    let reopened = Store::open(dir.path()).unwrap();
    assert_eq!(sequence_pointer(&reopened, Table::Schools), before_seq);
    assert_eq!(rows_held(&reopened, Table::Schools), before_rows);
    assert!(reopened.journal_keys("unit").unwrap().is_empty());
    assert_eq!(
        reopened.scan::<CanonicalSchool>(Table::Schools).unwrap(),
        vec![kept]
    );
}
#[test]
fn a_page_of_work_commits_rows_across_tables_and_the_journal_together() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let meet = CanonicalMeet::new(
        None,
        "Batch Invitational",
        "2026-06-01",
        CompetitionLevel::Invitational,
    );
    let mut batch = store.write_batch();
    batch
        .append_many(Table::Schools, std::slice::from_ref(&school("Batch High")))
        .unwrap();
    batch
        .append_many(Table::Meets, std::slice::from_ref(&meet))
        .unwrap();
    batch
        .append_many(Table::Schools, &[school("Second High")])
        .unwrap();
    batch
        .journal_done("unit", "batch-1", &serde_json::json!({ "rows": 3 }))
        .unwrap();
    assert!(!batch.is_empty(), "three rows and one entry are a page");
    batch.commit().unwrap();

    assert_eq!(rows_held(&store, Table::Schools), 2);
    assert_eq!(rows_held(&store, Table::Meets), 1);
    assert!(
        store.journal_keys("unit").unwrap().contains("batch-1"),
        "the entry the page named is in the journal"
    );
    drop(store);

    let reopened = Store::open(dir.path()).unwrap();
    assert_eq!(
        reopened
            .scan::<CanonicalSchool>(Table::Schools)
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        reopened.scan::<CanonicalMeet>(Table::Meets).unwrap().len(),
        1
    );
    assert!(reopened.journal_keys("unit").unwrap().contains("batch-1"));
}

#[test]
fn a_refused_entry_leaves_the_page_unwritten_and_every_counter_where_it_was() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let before = sequence_pointer(&store, Table::Schools);
    let mut batch = store.write_batch();
    batch
        .append_many(Table::Schools, &[school("Never Landed")])
        .unwrap();
    let refused = batch.journal_done("unit", "too-big", &"x".repeat(MAX_JOURNAL_VALUE_BYTES + 1));
    assert!(refused.is_err(), "an entry past its ceiling is refused");
    drop(batch);

    assert_eq!(
        sequence_pointer(&store, Table::Schools),
        before,
        "no reservation moved"
    );
    assert_eq!(rows_held(&store, Table::Schools), 0);
    assert!(store.journal_keys("unit").unwrap().is_empty());
}
