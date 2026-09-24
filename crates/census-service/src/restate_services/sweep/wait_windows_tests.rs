#[cfg(test)]
mod tests {
    use super::super::*;
    use std::cell::{Cell, RefCell};

    /// A scripted stand-in for the durable wait. It records every window it was asked to wait — the
    /// assertion that matters, since a loop that stopped waiting cannot fail a test that only counts
    /// windows.
    struct ScriptedWindows {
        /// The signal arrives after this many successful (elapsed) windows.
        /// Once the signal arrives, window() returns false immediately without recording.
        signal_after: u32,
        /// How many successful (true) windows have elapsed.
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
        async fn window(&self, seconds: u64) -> bool {
            Self::window(self, seconds).await
        }
    }

    /// A zero-second window is a wait of zero length, not a skipped wait. The pre-fix code awaited
    /// `ctx.signal` alone for `window_seconds == 0`, so with no signal the future never resolved and
    /// the call never returned; a later repair deleted the sleep entirely. That is why this asserts
    /// the waits themselves: three windows must mean three waits.
    #[tokio::test]
    async fn a_zero_second_window_waits_once_per_window() {
        let waits = ScriptedWindows::new(100); // no signal
        let (observed, interrupted) = Sweep::wait_windows_with(&waits, 3, 0).await;
        assert_eq!(observed, 3);
        assert!(!interrupted);
        assert_eq!(waits.windows_waited(), vec![0, 0, 0]);
    }

    /// A stop signal that has already arrived ends the sweep before the first window completes.
    #[tokio::test]
    async fn an_arrived_signal_cuts_the_first_window_short() {
        let waits = ScriptedWindows::new(0);
        let (observed, interrupted) = Sweep::wait_windows_with(&waits, 3, 1).await;
        // Signal has already arrived — no windows elapsed, no waits recorded.
        assert_eq!(observed, 0);
        assert!(interrupted);
        assert_eq!(waits.windows_waited(), Vec::<u64>::new());
    }

    /// A signal arriving mid-sweep keeps the windows already observed.
    #[tokio::test]
    async fn a_signal_mid_sweep_keeps_the_windows_observed_so_far() {
        let waits = ScriptedWindows::new(2);
        let (observed, interrupted) = Sweep::wait_windows_with(&waits, 5, 10).await;
        assert_eq!(observed, 2);
        assert!(interrupted);
        assert_eq!(waits.windows_waited(), vec![10, 10]);
    }

    /// Every window carries the requested duration, and exhausting them is not an interruption.
    #[tokio::test]
    async fn all_windows_elapse_without_a_signal() {
        let waits = ScriptedWindows::new(100); // no signal
        let (observed, interrupted) = Sweep::wait_windows_with(&waits, 5, 30).await;
        assert_eq!(observed, 5);
        assert!(!interrupted);
        assert_eq!(waits.windows_waited(), vec![30, 30, 30, 30, 30]);
    }

    /// No windows means no waits at all.
    #[tokio::test]
    async fn zero_windows_waits_for_nothing() {
        let waits = ScriptedWindows::new(0);
        let (observed, interrupted) = Sweep::wait_windows_with(&waits, 0, 5).await;
        assert_eq!(observed, 0);
        assert!(!interrupted);
        assert!(waits.windows_waited().is_empty());
    }
}
