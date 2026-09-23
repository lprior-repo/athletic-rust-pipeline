//! The drain certificate: what a region accepted, and how every unit of it ended.
//!
//! Root counterpart of the census supervisor's report (`crates/midwest-census/src/bootstrap.rs`)
//! and of its outcome lattice (`crates/midwest-census/src/outcome.rs`), so the two crates stay
//! comparable. The field set is frozen — `accepted`, `completed`, `cancelled`, `timed_out`,
//! `aborted`, `panicked`, `remaining` — and deliberately has no `stop_reason`: the root's stop is
//! the signal that already resolved `serve`'s `select!`.
//!
//! Every accepted unit lands in exactly one bucket: `completed` when the unit reached a terminal
//! state of its own — **`Ok(Err(reason))` counts as completed**, because the unit did finish and
//! did report, so the caller logs the reason and the same unit is not counted a second time as
//! unfinished — `cancelled` when it was dropped before finishing without unwinding; `panicked`
//! when it unwound; `aborted` when the drain killed it; `timed_out` when a deadline passed —
//! either the unit's own (`Outcome::Timeout`) or the drain's, in which case the abandoned units
//! stay in `remaining` too, making `remaining` the residual set rather than a terminal state.
//!
//! A correct drain preserves `completed + cancelled + aborted + panicked + remaining == accepted`.
//!
//! Regions nest. The runtime drains the browser region and the blocking pool, and the browser
//! actor certifies its own observers and fetch jobs: `merge` folds the inner counts into the outer
//! certificate, so one report at the end accounts for every unit either region accepted.

use std::time::Duration;
use tokio::task::JoinError;

mod counting;
pub use counting::DrainCounts;

/// Wall-clock budget for one region drain.
///
/// Bounded by construction: a blocking action has no cancellation point, so a drain that waited
/// for the pool forever could never report. Census bounds its own drain the same way.
pub const DRAIN_TIMEOUT: Duration = Duration::from_secs(30);

/// What one drained region accepted, and how each unit ended.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DrainReport {
    /// Units the region took responsibility for.
    pub accepted: u64,
    /// Units that reached a terminal state of their own, including those reporting an error.
    pub completed: u64,
    /// Units dropped or aborted before finishing, without unwinding.
    pub cancelled: u64,
    /// Units a deadline gave up on: the unit's own, or the drain's.
    pub timed_out: u64,
    /// Units the drain killed itself.
    pub aborted: u64,
    /// Units that unwound.
    pub panicked: u64,
    /// Units accepted, still unresolved when the drain stopped waiting.
    pub remaining: u64,
}

/// Saturating increment: a counter that wrapped would be a lie.
fn bump(counter: &mut u64, by: u64) -> u64 {
    *counter = counter.saturating_add(by);
    *counter
}

/// A container length as a counter: a length wider than `u64` saturates instead of wrapping.
pub fn count(len: usize) -> u64 {
    u64::try_from(len).unwrap_or(u64::MAX)
}

impl DrainReport {
    /// Take responsibility for `by` more units.
    pub fn accept(&mut self, by: u64) {
        bump(&mut self.accepted, by);
    }

    /// Count `by` units that finished, whether they reported success or an error of their own.
    pub fn complete(&mut self, by: u64) {
        bump(&mut self.completed, by);
    }

    /// Record one nested unit's terminal classification.
    pub fn record<T, E>(&mut self, outcome: &Outcome<T, E>) {
        match outcome {
            Outcome::Ok(_) | Outcome::Err(_) => {
                self.complete(1);
            }
            Outcome::Cancelled => {
                bump(&mut self.cancelled, 1);
            }
            Outcome::Timeout => {
                bump(&mut self.timed_out, 1);
            }
            Outcome::Panicked => {
                bump(&mut self.panicked, 1);
            }
        }
    }

    /// Count the units this drain killed itself.
    pub fn abort(&mut self, by: u64) {
        bump(&mut self.aborted, by);
    }

    /// Count `by` units that were dropped before finishing without unwinding.
    pub fn cancel(&mut self, by: u64) {
        bump(&mut self.cancelled, by);
    }

    /// Record one nested unit's join-level classification.
    pub fn record_state(&mut self, state: DrainState) {
        match state {
            DrainState::Completed => self.record(&Outcome::<(), ()>::Ok(())),
            DrainState::Cancelled => self.record(&Outcome::<(), ()>::Cancelled),
            DrainState::Panicked => self.record(&Outcome::<(), ()>::Panicked),
        }
    }

    /// Record one unit the drain stopped waiting for, from its join result.
    ///
    /// A unit that finished on its own keeps its own state; one the drain actually killed is
    /// counted as `aborted`, because that is what happened to it (`JoinError` cannot tell an
    /// explicit abort from a dropped future, and the drain performing the reap is the only thing
    /// that can).
    pub fn record_killed(&mut self, result: Result<(), JoinError>) {
        match DrainState::from_join(result) {
            DrainState::Cancelled => self.abort(1),
            state => self.record_state(state),
        }
    }

    /// Mark `by` units that already count as `remaining` as timed out as well.
    ///
    /// The blocking pool reports its unresolved units through its own `open` counter, so a drain
    /// deadline adds the `timed_out` reading without touching the residual those units already
    /// contributed.
    pub fn time_out(&mut self, by: u64) {
        bump(&mut self.timed_out, by);
    }

    /// Give up on `by` units that were never classified: accepted, unresolved, and remaining.
    ///
    /// A region that accepted units it never reaped — observers left in a join set when the drain's
    /// deadline passed — contributes them here, so the certificate carries both readings and the
    /// residual set stays honest.
    pub fn abandon(&mut self, by: u64) {
        bump(&mut self.remaining, by);
        bump(&mut self.timed_out, by);
    }

    /// Fold a nested region's certificate into this one.
    pub fn merge(&mut self, other: &Self) {
        for (counter, value) in [
            (&mut self.accepted, other.accepted),
            (&mut self.completed, other.completed),
            (&mut self.cancelled, other.cancelled),
            (&mut self.timed_out, other.timed_out),
            (&mut self.aborted, other.aborted),
            (&mut self.panicked, other.panicked),
            (&mut self.remaining, other.remaining),
        ] {
            bump(counter, value);
        }
    }
}

/// The five possible outcomes of an async boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome<T, E> {
    /// The unit completed successfully.
    Ok(T),
    /// The unit returned an application error — terminal, and counted as completed.
    Err(E),
    /// The unit was dropped or aborted before finishing.
    Cancelled,
    /// The unit reported that it gave up on a deadline.
    Timeout,
    /// The unit unwound.
    Panicked,
}

impl<T, E> Outcome<T, E> {
    /// Classify a join result from `JoinSet::join_next` or `spawn_blocking`.
    ///
    /// `Ok(Ok(v))` is `Ok(v)`, `Ok(Err(e))` is `Err(e)`, and a `JoinError` is `Panicked` when the
    /// task unwound and `Cancelled` otherwise. `Timeout` never comes from here: it is what a
    /// unit's own timeout wrapper reports.
    pub fn from_join(inner: Result<Result<T, E>, JoinError>) -> Self {
        match inner {
            Ok(Ok(value)) => Self::Ok(value),
            Ok(Err(error)) => Self::Err(error),
            Err(join) if join.is_panic() => Self::Panicked,
            Err(_) => Self::Cancelled,
        }
    }
}

/// Join-level classification of a unit whose region carries no inner result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrainState {
    /// The unit finished.
    Completed,
    /// The unit was dropped or aborted before finishing.
    Cancelled,
    /// The unit unwound.
    Panicked,
}

impl DrainState {
    /// Classify one `JoinSet::join_next` result.
    ///
    /// Unlike the census spelling this takes the non-optional result: `None` means the set is
    /// empty, which is the caller's loop condition rather than a state, and census reaches it with
    /// `unreachable!`.
    pub fn from_join(inner: Result<(), JoinError>) -> Self {
        match inner {
            Ok(()) => Self::Completed,
            Err(join) if join.is_panic() => Self::Panicked,
            Err(_) => Self::Cancelled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::task::JoinSet;

    #[tokio::test]
    async fn a_join_failure_classifies_as_panicked_or_cancelled() {
        let mut set = JoinSet::new();
        set.spawn(async { panic!("boom") });
        let panicked = set.join_next().await.expect("a task");
        set.spawn(async { std::future::pending::<()>().await });
        set.abort_all();
        let cancelled = set.join_next().await.expect("a task");

        assert_eq!(DrainState::from_join(panicked), DrainState::Panicked);
        assert_eq!(DrainState::from_join(cancelled), DrainState::Cancelled);
        assert_eq!(DrainState::from_join(Ok(())), DrainState::Completed);
        assert_eq!(Outcome::from_join(Ok(Ok::<u8, u8>(7))), Outcome::Ok(7));
        assert_eq!(Outcome::from_join(Ok(Err::<u8, u8>(9))), Outcome::Err(9));
    }

    #[test]
    fn an_inner_error_is_completed_while_a_deadline_leaves_units_remaining() {
        let mut report = DrainReport::default();
        report.accept(3);
        report.record(&Outcome::<(), &str>::Ok(()));
        report.record(&Outcome::<(), &str>::Err("observer failed"));
        report.abort(1);
        assert_eq!(
            report,
            DrainReport {
                accepted: 3,
                completed: 2,
                aborted: 1,
                ..DrainReport::default()
            }
        );

        let mut nested = DrainReport {
            accepted: 2,
            remaining: 2,
            ..DrainReport::default()
        };
        nested.time_out(nested.remaining);
        assert_eq!(nested.timed_out, 2);
        report.merge(&nested);
        assert_eq!(report.accepted, 5);
        assert_eq!(report.completed, 2);
        assert_eq!(report.aborted, 1);
        assert_eq!(report.timed_out, 2);
        assert_eq!(report.remaining, 2);
    }
}
