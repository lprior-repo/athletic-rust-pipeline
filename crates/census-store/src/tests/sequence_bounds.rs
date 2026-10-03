use super::*;
use serde::{Serialize, Serializer};
use std::sync::Condvar;
use std::time::Duration;

fn seed_mark(root: &Path, mark: u64) -> TestResult {
    let store = Store::open(root)?;
    let mut batch = store.db.batch();
    store.put_mark(&mut batch, Table::Schools, mark);
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    Ok(())
}

fn at_boundary(root: &Path) -> TestResult<Store> {
    seed_mark(root, MAX_ROWS_PER_TABLE - 1)?;
    Ok(Store::open(root)?)
}

#[test]
fn the_last_legal_append_survives_reopen_and_the_next_is_refused() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = at_boundary(dir.path())?;
    let last = school("Last Legal");
    store.append(Table::Schools, &last)?;
    let ceiling = usize::try_from(MAX_ROWS_PER_TABLE)?;
    check!(matches!(store.append(Table::Schools, &school("One Past")),
        Err(StoreError::TooManyRows { table, max }) if table == "schools" && max == ceiling));
    check!(eq; sequence_pointer(&store, Table::Schools)?, MAX_ROWS_PER_TABLE);
    drop(store);
    let store = Store::open(dir.path())?;
    check!(eq; sequence_pointer(&store, Table::Schools)?, MAX_ROWS_PER_TABLE);
    check!(eq; rows_held(&store, Table::Schools)?, 1);
    check!(eq; store.scan::<CanonicalSchool>(Table::Schools)?, vec![last]);
    Ok(())
}

#[test]
fn a_straddling_batch_changes_neither_sequence_nor_rows() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = at_boundary(dir.path())?;
    let rows = [school("Penultimate"), school("Last")];
    check!(matches!(store.append_many(Table::Schools, &rows),
        Err(StoreError::TooManyRows { table, .. }) if table == "schools"));
    drop(store);
    let store = Store::open(dir.path())?;
    check!(eq; sequence_pointer(&store, Table::Schools)?, MAX_ROWS_PER_TABLE - 1);
    check!(eq; rows_held(&store, Table::Schools)?, 0);
    check!(store.scan::<CanonicalSchool>(Table::Schools)?.is_empty());
    Ok(())
}

#[test]
fn planning_does_not_consume_sequences_and_refuses_overflow() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = at_boundary(dir.path())?;
    let last = store.sequences.plan(Table::Schools, 1)?;
    check!(eq; last.base, MAX_ROWS_PER_TABLE - 1);
    check!(eq; last.mark, MAX_ROWS_PER_TABLE);
    check!(eq; store.sequences.plan(Table::Schools, 0)?.base, last.base);
    check!(matches!(
        store.sequences.plan(Table::Schools, 2),
        Err(StoreError::TooManyRows { .. })
    ));
    check!(matches!(
        store.sequences.plan(Table::Schools, u64::MAX),
        Err(StoreError::CounterOverflow)
    ));
    check!(eq; sequence_pointer(&store, Table::Schools)?, last.base);
    store.append(Table::Schools, &school("Still Fits"))?;
    check!(eq; sequence_pointer(&store, Table::Schools)?, last.mark);
    Ok(())
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
fn concurrent_appenders_recheck_the_ceiling_after_serialization() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = at_boundary(dir.path())?;
    let ready = (Mutex::new(0), Condvar::new());
    let left = Aligned {
        record: school("Left"),
        ready: &ready,
    };
    let right = Aligned {
        record: school("Right"),
        ready: &ready,
    };
    let outcomes = std::thread::scope(|scope| -> TestResult<_> {
        let first = scope.spawn(|| store.append(Table::Schools, &left));
        let second = scope.spawn(|| store.append(Table::Schools, &right));
        let first = first.join().map_err(|_| "first appender panicked");
        let second = second.join().map_err(|_| "second appender panicked");
        Ok([first?, second?])
    })?;
    check!(eq; outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);
    check!(eq; outcomes.iter().filter(|outcome| matches!(outcome, Err(StoreError::TooManyRows { .. }))).count(), 1);
    drop(store);
    let store = Store::open(dir.path())?;
    check!(eq; sequence_pointer(&store, Table::Schools)?, MAX_ROWS_PER_TABLE);
    check!(eq; rows_held(&store, Table::Schools)?, 1);
    let rows = store.scan::<CanonicalSchool>(Table::Schools)?;
    check!(eq; rows.len(), 1);
    check!(rows[0] == left.record || rows[0] == right.record);
    Ok(())
}

#[test]
fn reopening_refuses_an_over_ceiling_persisted_sequence() -> TestResult {
    let dir = tempfile::tempdir()?;
    seed_mark(dir.path(), MAX_ROWS_PER_TABLE + 1)?;
    check!(matches!(
        Store::open(dir.path()),
        Err(StoreError::TooManyRows { .. })
    ));
    Ok(())
}

#[test]
fn recovering_a_missing_mark_refuses_sequence_overflow() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let record = school("Corrupted Sequence");
    let mut batch = store.db.batch();
    batch.insert(
        &store.entities,
        observation_key(Table::Schools, record.id.as_str(), u64::MAX),
        serde_json::to_vec(&record)?,
    );
    batch.remove(&store.meta, crate::sequences::mark_key(Table::Schools));
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    drop(store);
    check!(matches!(
        Store::open(dir.path()),
        Err(StoreError::CounterOverflow)
    ));
    Ok(())
}
