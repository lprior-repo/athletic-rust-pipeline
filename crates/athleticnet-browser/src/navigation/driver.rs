use super::{BrowserError, Clock, Duration, Future};

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
    loop {
        let remaining = deadline.saturating_duration_since(clock.now_instant());
        if remaining.is_zero() {
            return Err(BrowserError::Timeout);
        }
        if steps.step(remaining).await? {
            return Ok(());
        }
    }
}
