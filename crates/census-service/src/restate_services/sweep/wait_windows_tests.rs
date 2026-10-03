#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::restate_services::tests::sdk_error;
    use std::cell::{Cell, RefCell};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    struct ScriptedWindows {
        signal_after: u32,
        elapsed: Cell<u32>,
        waited: RefCell<Vec<u64>>,
    }

    impl ScriptedWindows {
        fn new(signal_after: u32) -> Self {
            Self {
                signal_after,
                elapsed: Cell::new(0),
                waited: RefCell::new(Vec::new()),
            }
        }

        fn windows_waited(&self) -> Vec<u64> {
            self.waited.borrow().clone()
        }

        async fn window(&self, seconds: u64) -> bool {
            let done = self.elapsed.get() >= self.signal_after;
            if done {
                false
            } else {
                self.elapsed.set(self.elapsed.get() + 1);
                self.waited.borrow_mut().push(seconds);
                true
            }
        }
    }

    impl WindowWaits for ScriptedWindows {
        async fn window(&self, seconds: u64) -> Result<bool, HandlerError> {
            Ok(Self::window(self, seconds).await)
        }
    }

    #[test]
    fn a_zero_second_window_waits_once_per_window() -> TestResult {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                let waits = ScriptedWindows::new(100);
                let (observed, interrupted) = Sweep::wait_windows_with(&waits, 3, 0)
                    .await
                    .map_err(sdk_error)?;
                check!(eq; observed, 3);
                check!(!interrupted);
                check!(eq; waits.windows_waited(), vec![0, 0, 0]);
                Ok(())
            })
    }

    #[test]
    fn an_arrived_signal_cuts_the_first_window_short() -> TestResult {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                let waits = ScriptedWindows::new(0);
                let (observed, interrupted) = Sweep::wait_windows_with(&waits, 3, 1)
                    .await
                    .map_err(sdk_error)?;
                check!(eq; observed, 0);
                check!(interrupted);
                check!(eq; waits.windows_waited(), Vec::<u64>::new());
                Ok(())
            })
    }

    #[test]
    fn a_signal_mid_sweep_keeps_the_windows_observed_so_far() -> TestResult {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                let waits = ScriptedWindows::new(2);
                let (observed, interrupted) = Sweep::wait_windows_with(&waits, 5, 10)
                    .await
                    .map_err(sdk_error)?;
                check!(eq; observed, 2);
                check!(interrupted);
                check!(eq; waits.windows_waited(), vec![10, 10]);
                Ok(())
            })
    }

    #[test]
    fn all_windows_elapse_without_a_signal() -> TestResult {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                let waits = ScriptedWindows::new(100);
                let (observed, interrupted) = Sweep::wait_windows_with(&waits, 5, 30)
                    .await
                    .map_err(sdk_error)?;
                check!(eq; observed, 5);
                check!(!interrupted);
                check!(eq; waits.windows_waited(), vec![30, 30, 30, 30, 30]);
                Ok(())
            })
    }

    #[test]
    fn zero_windows_waits_for_nothing() -> TestResult {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                let waits = ScriptedWindows::new(0);
                let (observed, interrupted) = Sweep::wait_windows_with(&waits, 0, 5)
                    .await
                    .map_err(sdk_error)?;
                check!(eq; observed, 0);
                check!(!interrupted);
                check!(waits.windows_waited().is_empty());
                Ok(())
            })
    }
}
