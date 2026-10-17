use std::time::Duration;
use tokio::task::JoinError;
mod report;

pub const DRAIN_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DrainReport {
    pub accepted: u64,
    pub completed: u64,
    pub cancelled: u64,
    pub timed_out: u64,
    pub aborted: u64,
    pub panicked: u64,
    pub remaining: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DrainAccountingError {
    #[error("browser drain accounting overflow")]
    Overflow,
}

pub(crate) fn count(len: usize) -> Result<u64, DrainAccountingError> {
    u64::try_from(len).map_err(|_| DrainAccountingError::Overflow)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome<T, E> {
    Ok(T),
    Err(E),
    Cancelled,
    Timeout,
    Panicked,
}

impl<T, E> Outcome<T, E> {
    pub fn from_join(inner: Result<Result<T, E>, JoinError>) -> Self {
        match inner {
            Ok(Ok(value)) => Self::Ok(value),
            Ok(Err(error)) => Self::Err(error),
            Err(join) if join.is_panic() => Self::Panicked,
            Err(_) => Self::Cancelled,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrainState {
    Completed,
    Cancelled,
    Panicked,
}

impl DrainState {
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

    fn panic_task_fault() {
        panic!("boom");
    }

    #[test]
    fn a_join_failure_classifies_as_panicked_or_cancelled() -> Result<(), Box<dyn std::error::Error>>
    {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                let mut set = JoinSet::new();
                set.spawn(async { panic_task_fault() });
                let panicked = set
                    .join_next()
                    .await
                    .ok_or("missing panicked task outcome")?;
                set.spawn(async { std::future::pending::<()>().await });
                set.abort_all();
                let cancelled = set
                    .join_next()
                    .await
                    .ok_or("missing cancelled task outcome")?;

                check!(eq; DrainState::from_join(panicked), DrainState::Panicked);
                check!(eq; DrainState::from_join(cancelled), DrainState::Cancelled);
                check!(eq; DrainState::from_join(Ok(())), DrainState::Completed);
                check!(eq; Outcome::from_join(Ok(Ok::<u8, u8>(7))), Outcome::Ok(7));
                check!(eq; Outcome::from_join(Ok(Err::<u8, u8>(9))), Outcome::Err(9));
                Ok(())
            })
    }
}
