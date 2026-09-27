use loom::sync::atomic::{AtomicU64, Ordering};
use loom::sync::{Arc, Mutex};
use loom::thread;

use super::sequences::{Reserved, SequenceCounter, SequenceValue};
use super::{StoreError, StoreResult, Table, MAX_ROWS_PER_TABLE};

impl SequenceValue for AtomicU64 {
    fn starting_at(start: u64) -> Self {
        AtomicU64::new(start)
    }

    fn publish(&self, mark: u64) {
        self.store(mark, Ordering::Relaxed);
    }

    fn next(&self) -> u64 {
        self.load(Ordering::Relaxed)
    }
}

struct Model {
    append: Mutex<()>,
    sequence: SequenceCounter<AtomicU64>,
}

fn model(start: u64) -> Arc<Model> {
    Arc::new(Model {
        append: Mutex::new(()),
        sequence: SequenceCounter::starting_at(start),
    })
}

fn commit(state: &Arc<Model>, count: u64) -> thread::JoinHandle<StoreResult<Reserved>> {
    let state = Arc::clone(state);
    thread::spawn(move || {
        let _append = state.append.lock().unwrap();
        let range = state.sequence.plan(Table::Schools, count)?;
        state.sequence.publish(range.mark);
        Ok(range)
    })
}

#[test]
fn committed_writers_tile_the_sequence_space() {
    let mut builder = loom::model::Builder::new();
    builder.max_threads = 4;
    builder.preemption_bound = Some(2);
    builder.check(|| {
        let state = model(0);
        let first = commit(&state, 2);
        let empty = commit(&state, 0);
        let second = commit(&state, 3);
        let mut ranges = [first, empty, second].map(|writer| {
            let range = writer.join().unwrap().unwrap();
            (range.base, range.mark - range.base)
        });
        ranges.sort_unstable();
        let end = ranges.into_iter().fold(0, |end, (base, count)| {
            assert_eq!(base, end);
            end + count
        });
        assert_eq!(end, 5);
        assert_eq!(state.sequence.next(), end);
    });
}

#[test]
fn competing_writers_cannot_cross_the_row_ceiling() {
    let mut builder = loom::model::Builder::new();
    builder.max_threads = 3;
    builder.preemption_bound = Some(2);
    builder.check(|| {
        let state = model(MAX_ROWS_PER_TABLE - 1);
        let left = commit(&state, 1);
        let right = commit(&state, 1);
        let outcomes = [left, right].map(|writer| writer.join().unwrap());
        assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);
        assert_eq!(
            outcomes
                .iter()
                .filter(|outcome| matches!(outcome, Err(StoreError::TooManyRows { .. })))
                .count(),
            1
        );
        assert_eq!(state.sequence.next(), MAX_ROWS_PER_TABLE);
    });
}

#[test]
fn an_unpublished_plan_leaves_no_sequence_gap() {
    let mut builder = loom::model::Builder::new();
    builder.max_threads = 3;
    builder.preemption_bound = Some(2);
    builder.check(|| {
        let state = model(0);
        let abandoned = Arc::clone(&state);
        let planner = thread::spawn(move || {
            let _append = abandoned.append.lock().unwrap();
            abandoned.sequence.plan(Table::Schools, 2)
        });
        let writer = commit(&state, 3);
        let planned = planner.join().unwrap().unwrap();
        let committed = writer.join().unwrap().unwrap();
        assert!(planned.base == 0 || planned.base == 3);
        assert_eq!(committed.base, 0);
        assert_eq!(committed.mark, 3);
        assert_eq!(state.sequence.next(), 3);
    });
}
