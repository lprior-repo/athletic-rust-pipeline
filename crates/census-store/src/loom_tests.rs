//! Model checks for the store's sequence allocation.
//!
//! The property the keyspace rests on — two writers never share a sequence — belongs to one atomic
//! reservation, so it is checked on that reservation: the same [`SequenceCounter`] the store
//! reserves through, instantiated with `loom`'s instrumented atomic, run over the schedules `loom`
//! explores. Nothing here opens a store: the keyspace, the batch and the durability mode are
//! fjall's, and the one thing concurrent writers contend on is the counter.
//!
//! What the reservations are checked for is exactly what the keys need: the runs tile the sequence
//! space — each starting where the last ended, so no sequence is handed out twice and none is
//! skipped — in every interleaving the model explores. A counter with one sequence left is checked
//! too: the reservation that cannot fit must be refused rather than wrap onto a sequence already
//! spent, because the row written under a spent sequence would overwrite the observation stored there.

use loom::sync::atomic::{AtomicU64, Ordering};
use loom::sync::Arc;
use loom::thread;

use super::sequences::{Reserve, SequenceCounter};

/// `loom`'s half of [`Reserve`]: the same reservation, on an atomic the model can schedule.
impl Reserve for AtomicU64 {
    fn starting_at(start: u64) -> Self {
        AtomicU64::new(start)
    }

    fn reserve(&self, count: u64) -> Option<u64> {
        let mut current = self.load(Ordering::Relaxed);
        loop {
            let next = current.checked_add(count)?;
            match self.compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => return Some(current),
                Err(observed) => current = observed,
            }
        }
    }

    fn next(&self) -> u64 {
        self.load(Ordering::Relaxed)
    }
}

#[test]
fn concurrent_writers_tile_the_sequence_space() {
    let mut builder = loom::model::Builder::new();
    builder.max_threads = 4;
    builder.preemption_bound = Some(2);
    builder.check(|| {
        let counter = Arc::new(SequenceCounter::<AtomicU64>::starting_at(0));
        let writer = |count: u64| {
            let counter = Arc::clone(&counter);
            thread::spawn(move || (counter.reserve(count), count))
        };
        let first = writer(2);
        let empty = writer(0);
        let second = writer(3);

        let mut reservations = [
            first.join().expect("writer finished"),
            empty.join().expect("writer finished"),
            second.join().expect("writer finished"),
        ]
        .map(|(start, count)| {
            (
                start.expect("a run this far from the counter's end always fits"),
                count,
            )
        });
        reservations.sort_unstable();

        let mut next = 0;
        for (start, count) in reservations {
            assert_eq!(
                start, next,
                "a reservation did not continue where the previous one ended"
            );
            next += count;
        }
        assert_eq!(
            next, 5,
            "the counter handed out a sequence nobody asked for"
        );
        assert_eq!(
            counter.next(),
            5,
            "the counter does not sit at the end of what it handed out"
        );
    });
}

#[test]
fn a_reservation_that_would_wrap_is_refused() {
    let mut builder = loom::model::Builder::new();
    builder.max_threads = 3;
    builder.preemption_bound = Some(2);
    builder.check(|| {
        let counter = Arc::new(SequenceCounter::<AtomicU64>::starting_at(u64::MAX - 1));
        let writer = |count: u64| {
            let counter = Arc::clone(&counter);
            thread::spawn(move || counter.reserve(count))
        };
        let single = writer(1);
        let pair = writer(2);

        let reservations = [
            single.join().expect("writer finished"),
            pair.join().expect("writer finished"),
        ];
        assert_eq!(
            reservations.iter().filter(|start| start.is_some()).count(),
            1,
            "exactly one of the two runs fits in the counter's last sequence: {reservations:?}"
        );
        assert_eq!(
            counter.next(),
            u64::MAX,
            "the counter did not stop at the last sequence it handed out"
        );
    });
}
