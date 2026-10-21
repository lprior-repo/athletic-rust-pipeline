use super::{BrowserError, Clock, Duration, Future, MAX_ITERATIONS};

pub(super) trait NavigationSteps: Send {
    fn step(
        &mut self,
        remaining: Duration,
    ) -> impl Future<Output = Result<bool, BrowserError>> + Send;
}

pub(super) async fn drive_events<F>(
    clock: &dyn Clock,
    deadline: tokio::time::Instant,
    steps: &mut F,
) -> Result<(), BrowserError>
where
    F: NavigationSteps,
{
    let mut iterations: u64 = 0;
    loop {
        iterations = iterations.saturating_add(1);
        if iterations > MAX_ITERATIONS {
            tracing::warn!(iterations, "navigation loop reached iteration bound");
            return Err(BrowserError::Timeout);
        }
        let remaining = deadline.saturating_duration_since(clock.now_instant());
        if remaining.is_zero() {
            return Err(BrowserError::Timeout);
        }
        if steps.step(remaining).await? {
            return Ok(());
        }
    }
}
