use super::*;
use serde::{Serialize, Serializer};
use std::sync::Condvar;
use std::time::Duration;

fn seed_mark(root: &Path, mark: u64) {
    let store = Store::open(root).unwrap();
    let mut batch = store.db.batch();
    store.put_mark(&mut batch, Table::Schools, mark);
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .unwrap();
}

fn at_boundary(root: &Path) -> Store {
    seed_mark(root, MAX_ROWS_PER_TABLE - 1);
    Store::open(root).unwrap()
}

#[test]
fn the_last_legal_append_survives_reopen_and_the_next_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let store = at_boundary(dir.path());
    let last = school("Last Legal");
    store.append(Table::Schools, &last).unwrap();
    assert!(matches!(store.append(Table::Schools, &school("One Past")),
        Err(StoreError::TooManyRows { table, max })
            if table == "schools" && max == usize::try_from(MAX_ROWS_PER_TABLE).unwrap()));
    assert_eq!(sequence_pointer(&store, Table::Schools), MAX_ROWS_PER_TABLE);
    drop(store);
    let store = Store::open(dir.path()).unwrap();
    assert_eq!(sequence_pointer(&store, Table::Schools), MAX_ROWS_PER_TABLE);
    assert_eq!(rows_held(&store, Table::Schools), 1);
    assert_eq!(
        store.scan::<CanonicalSchool>(Table::Schools).unwrap(),
        vec![last]
    );
}

#[test]
fn a_straddling_batch_changes_neither_sequence_nor_rows() {
    let dir = tempfile::tempdir().unwrap();
    let store = at_boundary(dir.path());
    let rows = [school("Penultimate"), school("Last")];
    assert!(matches!(store.append_many(Table::Schools, &rows),
        Err(StoreError::TooManyRows { table, .. }) if table == "schools"));
    drop(store);
    let store = Store::open(dir.path()).unwrap();
    assert_eq!(
        sequence_pointer(&store, Table::Schools),
        MAX_ROWS_PER_TABLE - 1
    );
    assert_eq!(rows_held(&store, Table::Schools), 0);
    assert!(store
        .scan::<CanonicalSchool>(Table::Schools)
        .unwrap()
        .is_empty());
}

#[test]
fn planning_does_not_consume_sequences_and_refuses_overflow() {
    let dir = tempfile::tempdir().unwrap();
    let store = at_boundary(dir.path());
    let last = store.sequences.plan(Table::Schools, 1).unwrap();
    assert_eq!(last.base, MAX_ROWS_PER_TABLE - 1);
    assert_eq!(last.mark, MAX_ROWS_PER_TABLE);
    assert_eq!(
        store.sequences.plan(Table::Schools, 0).unwrap().base,
        last.base
    );
    assert!(matches!(
        store.sequences.plan(Table::Schools, 2),
        Err(StoreError::TooManyRows { .. })
    ));
    assert!(matches!(
        store.sequences.plan(Table::Schools, u64::MAX),
        Err(StoreError::CounterOverflow)
    ));
    assert_eq!(sequence_pointer(&store, Table::Schools), last.base);
    store.append(Table::Schools, &school("Still Fits")).unwrap();
    assert_eq!(sequence_pointer(&store, Table::Schools), last.mark);
}

struct Aligned<'a> {
    record: CanonicalSchool,
    ready: &'a (Mutex<usize>, Condvar),
}

impl Serialize for Aligned<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let (lock, signal) = self.ready;
        let mut ready = lock.lock().map_err(serde::ser::Error::custom)?;
        *ready += 1;
        signal.notify_all();
        let (ready, _) = signal
            .wait_timeout_while(ready, Duration::from_secs(30), |count| *count < 2)
            .map_err(serde::ser::Error::custom)?;
        if *ready != 2 {
            return Err(serde::ser::Error::custom(
                "second serializer did not arrive",
            ));
        }
        drop(ready);
        self.record.serialize(serializer)
    }
}

#[test]
fn concurrent_appenders_recheck_the_ceiling_after_serialization() {
    let dir = tempfile::tempdir().unwrap();
    let store = at_boundary(dir.path());
    let ready = (Mutex::new(0), Condvar::new());
    let left = Aligned {
        record: school("Left"),
        ready: &ready,
    };
    let right = Aligned {
        record: school("Right"),
        ready: &ready,
    };
    let outcomes = std::thread::scope(|scope| {
        let first = scope.spawn(|| store.append(Table::Schools, &left));
        let second = scope.spawn(|| store.append(Table::Schools, &right));
        [first.join().unwrap(), second.join().unwrap()]
    });
    assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(outcome, Err(StoreError::TooManyRows { .. })))
            .count(),
        1
    );
    drop(store);
    let store = Store::open(dir.path()).unwrap();
    assert_eq!(sequence_pointer(&store, Table::Schools), MAX_ROWS_PER_TABLE);
    assert_eq!(rows_held(&store, Table::Schools), 1);
    let rows = store.scan::<CanonicalSchool>(Table::Schools).unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0] == left.record || rows[0] == right.record);
}

#[test]
fn reopening_refuses_an_over_ceiling_persisted_sequence() {
    let dir = tempfile::tempdir().unwrap();
    seed_mark(dir.path(), MAX_ROWS_PER_TABLE + 1);
    assert!(matches!(
        Store::open(dir.path()),
        Err(StoreError::TooManyRows { .. })
    ));
}

#[test]
fn recovering_a_missing_mark_refuses_sequence_overflow() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let record = school("Corrupted Sequence");
    let mut batch = store.db.batch();
    batch.insert(
        &store.entities,
        observation_key(Table::Schools, record.id.as_str(), u64::MAX),
        serde_json::to_vec(&record).unwrap(),
    );
    batch.remove(&store.meta, crate::sequences::mark_key(Table::Schools));
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .unwrap();
    drop(store);
    assert!(matches!(
        Store::open(dir.path()),
        Err(StoreError::CounterOverflow)
    ));
}
