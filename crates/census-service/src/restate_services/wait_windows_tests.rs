#[cfg(test)]
mod wait_windows_tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    type TestResult<E = Box<dyn std::error::Error>> = Result<(), E>;

    struct ScriptedWindows {
        cut_short_at: Option<u32>,
        calls: Cell<u32>,
        waited: RefCell<Vec<u64>>,
    }

    impl ScriptedWindows {
        fn new(cut_short_at: Option<u32>) -> Self {
            Self {
                cut_short_at,
                calls: Cell::new(0),
                waited: RefCell::new(Vec::new()),
            }
        }

        fn windows_waited(&self) -> Vec<u64> {
            self.waited.borrow().clone()
        }
    }

    impl WindowWaits for ScriptedWindows {
        async fn window(&self, seconds: u64) -> bool {
            let index = self.calls.get();
            self.calls.set(index.saturating_add(1));
            self.waited.borrow_mut().push(seconds);
            self.cut_short_at != Some(index)
        }
    }

    #[test]
    fn a_zero_second_window_waits_once_per_window() -> TestResult {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
        let waits = ScriptedWindows::new(None);
        let (observed, interrupted) = Sweep::wait_windows_with(&waits, 3, 0).await;

        check!(eq; observed, 3, "all three windows are observed");
        check!(
            !interrupted,
            "no signal arrived, so nothing was interrupted"
        );
        check!(eq;
            waits.windows_waited(),
            vec![0, 0, 0],
            "every window is waited out, even at zero seconds"
        );
        Ok(())
            })
    }

    #[test]
    fn an_arrived_signal_cuts_the_first_window_short() -> TestResult {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
        let waits = ScriptedWindows::new(Some(0));
        let (observed, interrupted) = Sweep::wait_windows_with(&waits, 5, 10).await;

        check!(eq; observed, 0);
        check!(interrupted);
        check!(eq;
            waits.windows_waited().len(),
            1,
            "the sweep stops inside the first window"
        );
        Ok(())
            })
    }

    #[test]
    fn a_signal_mid_sweep_keeps_the_windows_observed_so_far() -> TestResult {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
        let waits = ScriptedWindows::new(Some(2));
        let (observed, interrupted) = Sweep::wait_windows_with(&waits, 5, 30).await;

        check!(eq; observed, 2);
        check!(interrupted);
        check!(eq; waits.windows_waited(), vec![30, 30, 30]);
        Ok(())
            })
    }

    #[test]
    fn all_windows_elapse_without_a_signal() -> TestResult {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
        let waits = ScriptedWindows::new(None);
        let (observed, interrupted) = Sweep::wait_windows_with(&waits, 3, 42).await;

        check!(eq; observed, 3);
        check!(!interrupted);
        check!(eq; waits.windows_waited(), vec![42, 42, 42]);
        Ok(())
            })
    }

    #[test]
    fn zero_windows_waits_for_nothing() -> TestResult {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
        let waits = ScriptedWindows::new(None);
        let (observed, interrupted) = Sweep::wait_windows_with(&waits, 0, 5).await;

        check!(eq; observed, 0);
        check!(!interrupted);
        check!(waits.windows_waited().is_empty());
        Ok(())
            })
    }
}
