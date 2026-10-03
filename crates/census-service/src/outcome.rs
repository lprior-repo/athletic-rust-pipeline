use tokio::task::JoinError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome<T, E> {
    Ok(T),
    Err(E),
    Cancelled,
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

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn panic_task_fault() {
        panic!("boom");
    }

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

    #[test]
    fn from_join_panic_is_panicked() -> TestResult {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                let mut set = JoinSet::new();
                set.spawn(async {
                    panic_task_fault();
                });
                let join = match set.join_next().await.ok_or("missing panicked task")? {
                    Err(join) => join,
                    Ok(_) => return Err("task did not panic".into()),
                };
                let inner: Result<Result<i32, &'static str>, JoinError> = Err(join);
                let outcome = Outcome::from_join(inner);
                if outcome != Outcome::Panicked {
                    return Err(format!("expected Outcome::Panicked, got {outcome:?}").into());
                }
                Ok(())
            })
    }

    #[test]
    fn from_join_cancelled_is_cancelled() -> TestResult {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                let mut set = JoinSet::new();
                set.spawn(async {
                    std::future::pending::<()>().await;
                });
                set.abort_all();
                let join = match set.join_next().await.ok_or("missing cancelled task")? {
                    Err(join) => join,
                    Ok(()) => return Err("task was not cancelled".into()),
                };
                let inner: Result<Result<i32, &'static str>, JoinError> = Err(join);
                let outcome = Outcome::from_join(inner);
                if outcome != Outcome::Cancelled {
                    return Err(format!("expected Outcome::Cancelled, got {outcome:?}").into());
                }
                Ok(())
            })
    }

    #[test]
    fn drain_state_completed() {
        assert_eq!(DrainState::from_join(Ok(())), DrainState::Completed);
    }

    #[test]
    fn drain_state_panicked_on_task_panic() -> TestResult {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                let mut set = JoinSet::new();
                set.spawn(async {
                    panic_task_fault();
                });
                let join = match set.join_next().await.ok_or("missing panicked task")? {
                    Err(join) => join,
                    Ok(_) => return Err("task did not panic".into()),
                };
                let state = DrainState::from_join(Err(join));
                if state != DrainState::Panicked {
                    return Err(format!("expected DrainState::Panicked, got {state:?}").into());
                }
                Ok(())
            })
    }

    #[test]
    fn drain_state_cancelled_on_abort() -> TestResult {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                let mut set = JoinSet::new();
                set.spawn(async {
                    std::future::pending::<()>().await;
                });
                set.abort_all();
                let join = match set.join_next().await.ok_or("missing cancelled task")? {
                    Err(join) => join,
                    Ok(()) => return Err("task was not cancelled".into()),
                };
                let state = DrainState::from_join(Err(join));
                if state != DrainState::Cancelled {
                    return Err(format!("expected DrainState::Cancelled, got {state:?}").into());
                }
                Ok(())
            })
    }
}
