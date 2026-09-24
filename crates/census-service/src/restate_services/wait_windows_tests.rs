#[cfg(test)]
mod wait_windows_tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    /// A scripted stand-in for the durable wait. It records every window it was asked to wait — the
    /// assertion that matters, since a loop that stopped waiting cannot fail a test that only counts
    /// windows.
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

    /// A zero-second window is a wait of zero length, not a skipped wait. The pre-fix code awaited
    /// `ctx.signal` alone for `window_seconds == 0`, so with no signal the future never resolved and
    /// the call never returned; a later repair deleted the sleep entirely. That is why this asserts
    /// the waits themselves: three windows must mean three waits.
    #[tokio::test]
    async fn a_zero_second_window_waits_once_per_window() {
        let waits = ScriptedWindows::new(None);
        let (observed, interrupted) = Sweep::wait_windows_with(&waits, 3, 0).await;

        assert_eq!(observed, 3, "all three windows are observed");
        assert!(
            !interrupted,
            "no signal arrived, so nothing was interrupted"
        );
        assert_eq!(
            waits.windows_waited(),
            vec![0, 0, 0],
            "every window is waited out, even at zero seconds"
        );
    }

    /// A stop signal that has already arrived ends the sweep inside its first window.
    #[tokio::test]
    async fn an_arrived_signal_cuts_the_first_window_short() {
        let waits = ScriptedWindows::new(Some(0));
        let (observed, interrupted) = Sweep::wait_windows_with(&waits, 5, 10).await;

        assert_eq!(observed, 0);
        assert!(interrupted);
        assert_eq!(
            waits.windows_waited().len(),
            1,
            "the sweep stops inside the first window"
        );
    }

    /// A signal arriving mid-sweep keeps the windows already observed.
    #[tokio::test]
    async fn a_signal_mid_sweep_keeps_the_windows_observed_so_far() {
        let waits = ScriptedWindows::new(Some(2));
        let (observed, interrupted) = Sweep::wait_windows_with(&waits, 5, 30).await;

        assert_eq!(observed, 2);
        assert!(interrupted);
        assert_eq!(waits.windows_waited(), vec![30, 30, 30]);
    }

    /// Every window carries the requested duration, and exhausting them is not an interruption.
    #[tokio::test]
    async fn all_windows_elapse_without_a_signal() {
        let waits = ScriptedWindows::new(None);
        let (observed, interrupted) = Sweep::wait_windows_with(&waits, 3, 42).await;

        assert_eq!(observed, 3);
        assert!(!interrupted);
        assert_eq!(waits.windows_waited(), vec![42, 42, 42]);
    }

    /// No windows means no waits at all.
    #[tokio::test]
    async fn zero_windows_waits_for_nothing() {
        let waits = ScriptedWindows::new(None);
        let (observed, interrupted) = Sweep::wait_windows_with(&waits, 0, 5).await;

        assert_eq!(observed, 0);
        assert!(!interrupted);
        assert!(waits.windows_waited().is_empty());
    }
}
