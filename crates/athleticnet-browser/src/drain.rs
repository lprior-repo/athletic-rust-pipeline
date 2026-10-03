use std::time::Duration;
use tokio::task::JoinError;

mod counting;
pub use counting::DrainCounts;

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

fn bump(counter: &mut u64, by: u64) -> u64 {
    *counter = counter.saturating_add(by);
    *counter
}

pub fn count(len: usize) -> u64 {
    u64::try_from(len).map_or(u64::MAX, |value| value)
}

impl DrainReport {
    pub fn accept(&mut self, by: u64) {
        bump(&mut self.accepted, by);
    }

    pub fn complete(&mut self, by: u64) {
        bump(&mut self.completed, by);
    }

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

    pub fn abort(&mut self, by: u64) {
        bump(&mut self.aborted, by);
    }

    pub fn cancel(&mut self, by: u64) {
        bump(&mut self.cancelled, by);
    }

    pub fn record_state(&mut self, state: DrainState) {
        match state {
            DrainState::Completed => self.record(&Outcome::<(), ()>::Ok(())),
            DrainState::Cancelled => self.record(&Outcome::<(), ()>::Cancelled),
            DrainState::Panicked => self.record(&Outcome::<(), ()>::Panicked),
        }
    }

    pub fn record_killed(&mut self, result: Result<(), JoinError>) {
        match DrainState::from_join(result) {
            DrainState::Cancelled => self.abort(1),
            state => self.record_state(state),
        }
    }

    pub fn time_out(&mut self, by: u64) {
        bump(&mut self.timed_out, by);
    }

    pub fn abandon(&mut self, by: u64) {
        bump(&mut self.remaining, by);
        bump(&mut self.timed_out, by);
    }

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

    #[test]
    fn an_inner_error_is_completed_while_a_deadline_leaves_units_remaining(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut report = DrainReport::default();
        report.accept(3);
        report.record(&Outcome::<(), &str>::Ok(()));
        report.record(&Outcome::<(), &str>::Err("observer failed"));
        report.abort(1);
        check!(eq; report,
        DrainReport {
            accepted: 3,
            completed: 2,
            aborted: 1,
            ..DrainReport::default()
        });

        let mut nested = DrainReport {
            accepted: 2,
            remaining: 2,
            ..DrainReport::default()
        };
        nested.time_out(nested.remaining);
        check!(eq; nested.timed_out, 2);
        report.merge(&nested);
        check!(eq; report.accepted, 5);
        check!(eq; report.completed, 2);
        check!(eq; report.aborted, 1);
        check!(eq; report.timed_out, 2);
        check!(eq; report.remaining, 2);
        Ok(())
    }
}
