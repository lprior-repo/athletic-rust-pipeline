use super::*;

struct ReadyEvents<'a> {
    clock: &'a TestClock,
    count: u32,
    complete_at: Option<u32>,
    advance_ms: u64,
    sentinel: u32,
}

impl NavigationSteps for ReadyEvents<'_> {
    async fn step(&mut self, _remaining: Duration) -> Result<bool, BrowserError> {
        self.count = self.count.checked_add(1).ok_or(BrowserError::Protocol)?;
        if self.count > self.sentinel {
            return Err(BrowserError::Protocol);
        }
        self.clock.advance_ms(self.advance_ms);
        Ok(self.complete_at == Some(self.count))
    }
}

#[test]
fn ready_event_volume_does_not_replace_navigation_deadline(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .start_paused(true)
        .build()?
        .block_on(async {
            let clock = TestClock::at(0);
            let start = clock.now_instant();
            let deadline = navigation_deadline(start, Duration::from_secs(1));
            let mut steps = ReadyEvents {
                clock: &clock,
                count: 0,
                complete_at: Some(1026),
                advance_ms: 0,
                sentinel: 1026,
            };
            drive_events(&clock, deadline, &mut steps).await?;
            check!(eq; steps.count, 1026);
            check!(eq; clock.now_instant(), start);
            Ok(())
        })
}

#[test]
fn ready_events_cannot_starve_monotonic_navigation_timeout(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .start_paused(true)
        .build()?
        .block_on(async {
            let clock = TestClock::at(0);
            let start = clock.now_instant();
            let deadline = navigation_deadline(start, Duration::from_secs(1));
            let mut steps = ReadyEvents {
                clock: &clock,
                count: 0,
                complete_at: None,
                advance_ms: 1,
                sentinel: 1024,
            };
            let result = drive_events(&clock, deadline, &mut steps).await;
            check!(matches!(result, Err(BrowserError::Timeout)));
            check!(eq; steps.count, 1000);
            check!(eq; clock.now_instant().duration_since(start), Duration::from_secs(1));
            Ok(())
        })
}
