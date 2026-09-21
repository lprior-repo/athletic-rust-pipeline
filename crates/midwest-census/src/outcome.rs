/// Outcome lattice at async boundaries: classifies `tokio::task::JoinError` and inner results
/// into a five-state enum so every boundary has a single, testable policy decision point.
///
/// The wire semantics are preserved: `Cancelled` stays terminal (a cancelled `ctx.run` is a
/// region shutdown or abort, not a retryable fault). `Panicked` is also terminal because
/// replaying the journal value that panicked would panic again.
use tokio::task::JoinError;

/// The five possible outcomes of an async boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome<T, E> {
    /// The job completed successfully.
    Ok(T),
    /// The job returned an application error.
    Err(E),
    /// The task was cancelled (abort, shutdown, or deadline).
    Cancelled,
    /// The task timed out before producing a result.
    Timeout,
    /// The task panicked.
    Panicked,
}

impl<T, E> Outcome<T, E> {
    /// Classify a `JoinResult` from `JoinSet::join_next()` or `spawn_blocking`.
    ///
    /// `Ok(Ok(v))` → `Ok(v)`. `Ok(Err(e))` → `Err(e)`. `Err(join)` → `Panicked` if
    /// `join.is_panic()` (replaying the same journal value would panic again), otherwise
    /// `Cancelled` (the task was aborted, shut down, or timed out — not a retryable fault).
    pub fn from_join(inner: Result<Result<T, E>, JoinError>) -> Self {
        match inner {
            Ok(Ok(value)) => Self::Ok(value),
            Ok(Err(error)) => Self::Err(error),
            Err(join) if join.is_panic() => Self::Panicked,
            Err(_) => Self::Cancelled,
        }
    }
}

/// Classification for drain reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrainState {
    /// Completed successfully.
    Completed,
    /// Cancelled (abort or shutdown).
    Cancelled,
    /// Panicked.
    Panicked,
}

impl DrainState {
    /// Classify one join result from `JoinSet::join_next()` for the drain report. The `None` arm
    /// ("no joinable task left") belongs to the caller's loop and never reaches this classifier,
    /// so this match is total over the results that do.
    pub fn from_join(joined: Result<(), JoinError>) -> Self {
        match joined {
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

    #[test]
    fn from_join_ok_wraps_value() {
        let inner: Result<Result<i32, &'static str>, JoinError> = Ok(Ok(42));
        let outcome = Outcome::from_join(inner);
        assert_eq!(outcome, Outcome::Ok(42));
    }

    #[test]
    fn from_join_err_wraps_error() {
        let inner: Result<Result<i32, &'static str>, JoinError> = Ok(Err("bad input"));
        let outcome = Outcome::from_join(inner);
        assert_eq!(outcome, Outcome::Err("bad input"));
    }

    #[tokio::test]
    async fn from_join_panic_is_panicked() {
        let mut set = JoinSet::new();
        set.spawn(async {
            panic!("boom");
        });
        let join = set.join_next().await.unwrap().unwrap_err();
        let inner: Result<Result<i32, &'static str>, JoinError> = Err(join);
        let outcome = Outcome::from_join(inner);
        assert_eq!(outcome, Outcome::Panicked);
    }

    #[tokio::test]
    async fn from_join_cancelled_is_cancelled() {
        let mut set = JoinSet::new();
        set.spawn(async {
            std::future::pending::<()>().await;
        });
        set.abort_all();
        let join = set.join_next().await.unwrap().unwrap_err();
        let inner: Result<Result<i32, &'static str>, JoinError> = Err(join);
        let outcome = Outcome::from_join(inner);
        assert_eq!(outcome, Outcome::Cancelled);
    }

    #[test]
    fn drain_state_completed() {
        assert_eq!(DrainState::from_join(Ok(())), DrainState::Completed);
    }

    #[tokio::test]
    async fn drain_state_panicked_on_task_panic() {
        let mut set = JoinSet::new();
        set.spawn(async {
            panic!("boom");
        });
        let join = set.join_next().await.unwrap().unwrap_err();
        assert_eq!(DrainState::from_join(Err(join)), DrainState::Panicked);
    }

    #[tokio::test]
    async fn drain_state_cancelled_on_abort() {
        let mut set = JoinSet::new();
        set.spawn(async {
            std::future::pending::<()>().await;
        });
        set.abort_all();
        let join = set.join_next().await.unwrap().unwrap_err();
        assert_eq!(DrainState::from_join(Err(join)), DrainState::Cancelled);
    }
}
