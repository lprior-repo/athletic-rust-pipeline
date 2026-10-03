use loom::sync::atomic::{AtomicU64, Ordering};
use loom::sync::{Arc, Mutex};
use loom::thread;

use super::sequences::{Reserved, SequenceCounter, SequenceValue};
use super::{StoreError, Table, MAX_ROWS_PER_TABLE};

type ModelResult<T> = Result<T, String>;

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

fn commit(
    state: &Arc<Model>,
    count: u64,
) -> thread::JoinHandle<ModelResult<Result<Reserved, StoreError>>> {
    let state = Arc::clone(state);
    thread::spawn(move || {
        let _append = state
            .append
            .lock()
            .map_err(|error| format!("append lock poisoned: {error}"))?;
        let range = match state.sequence.plan(Table::Schools, count) {
            Ok(range) => range,
            Err(error) => return Ok(Err(error)),
        };
        state.sequence.publish(range.mark);
        Ok(Ok(range))
    })
}

fn tiled_sequence_snapshot() -> ModelResult<(bool, u64, u64)> {
    let state = model(0);
    let first = commit(&state, 2);
    let empty = commit(&state, 0);
    let second = commit(&state, 3);
    let [first, empty, second] = [first, empty, second].map(|writer| {
        writer
            .join()
            .map_err(|_| "sequence writer panicked".to_string())
    });
    let mut ranges = [
        first??.map_err(|error| error.to_string())?,
        empty??.map_err(|error| error.to_string())?,
        second??.map_err(|error| error.to_string())?,
    ]
    .map(|range| (range.base, range.mark - range.base));
    ranges.sort_unstable();
    let (tiles, end) = ranges
        .into_iter()
        .fold((true, 0), |(tiles, end), (base, count)| {
            (tiles && base == end, end + count)
        });
    Ok((tiles, end, state.sequence.next()))
}

#[test]
fn committed_writers_tile_the_sequence_space() {
    let mut builder = loom::model::Builder::new();
    builder.max_threads = 4;
    builder.preemption_bound = Some(2);
    builder.check(|| assert_eq!(tiled_sequence_snapshot(), Ok((true, 5, 5))));
}

fn ceiling_snapshot() -> ModelResult<(usize, usize, u64)> {
    let state = model(MAX_ROWS_PER_TABLE - 1);
    let left = commit(&state, 1);
    let right = commit(&state, 1);
    let mut successes = 0;
    let mut refusals = 0;
    for writer in [left, right] {
        let outcome = writer
            .join()
            .map_err(|_| "ceiling writer panicked".to_string())??;
        match outcome {
            Ok(_) => successes += 1,
            Err(StoreError::TooManyRows { .. }) => refusals += 1,
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok((successes, refusals, state.sequence.next()))
}

#[test]
fn competing_writers_cannot_cross_the_row_ceiling() {
    let mut builder = loom::model::Builder::new();
    builder.max_threads = 3;
    builder.preemption_bound = Some(2);
    builder.check(|| assert_eq!(ceiling_snapshot(), Ok((1, 1, MAX_ROWS_PER_TABLE))));
}

fn unpublished_snapshot() -> ModelResult<(bool, u64, u64, u64)> {
    let state = model(0);
    let abandoned = Arc::clone(&state);
    let planner = thread::spawn(move || -> ModelResult<Reserved> {
        let _append = abandoned
            .append
            .lock()
            .map_err(|error| format!("planner lock poisoned: {error}"))?;
        abandoned
            .sequence
            .plan(Table::Schools, 2)
            .map_err(|error| error.to_string())
    });
    let writer = commit(&state, 3);
    let planned = planner
        .join()
        .map_err(|_| "sequence planner panicked".to_string())??;
    let committed = writer
        .join()
        .map_err(|_| "sequence writer panicked".to_string())??
        .map_err(|error| error.to_string())?;
    Ok((
        planned.base == 0 || planned.base == 3,
        committed.base,
        committed.mark,
        state.sequence.next(),
    ))
}

#[test]
fn an_unpublished_plan_leaves_no_sequence_gap() {
    let mut builder = loom::model::Builder::new();
    builder.max_threads = 3;
    builder.preemption_bound = Some(2);
    builder.check(|| assert_eq!(unpublished_snapshot(), Ok((true, 0, 3, 3))));
}
