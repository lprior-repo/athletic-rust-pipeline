use super::*;
use crate::clock::TestClock;
use std::cell::Cell;
use std::future::{ready, Ready};
use std::rc::Rc;

fn scripted(
    script: Vec<NavigationOutcome>,
) -> impl FnMut() -> Ready<Result<NavigationOutcome, BrowserError>> {
    let mut index = 0;
    move || {
        let outcome = script
            .get(index)
            .or_else(|| script.last())
            .cloned()
            .map_or(NavigationOutcome::Challenged, |value| value);
        index += 1;
        ready(Ok(outcome))
    }
}

#[test]
fn a_challenge_that_clears_itself_settles_on_the_page_it_became(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .start_paused(true)
        .build()?
        .block_on(async {
            let clock = TestClock::at(0);
            let started = tokio::time::Instant::now();
            let settled = settle(
                &clock,
                Duration::from_secs(30),
                scripted(vec![
                    NavigationOutcome::Challenged,
                    NavigationOutcome::Pending,
                    NavigationOutcome::Challenged,
                    NavigationOutcome::Ready,
                ]),
            )
            .await?;

            check!(eq; settled, NavigationOutcome::Ready);
            check!(eq; started.elapsed(), CHALLENGE_POLL * 3);
            Ok(())
        })
}

#[test]
fn a_challenge_that_outlasts_the_budget_is_reported_as_challenged(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .start_paused(true)
        .build()?
        .block_on(async {
            let clock = TestClock::at(0);
            let started = tokio::time::Instant::now();
            let settled = settle(
                &clock,
                Duration::from_secs(1),
                scripted(vec![NavigationOutcome::Challenged]),
            )
            .await?;

            check!(eq; settled, NavigationOutcome::Challenged);
            check!(eq; started.elapsed(),
Duration::from_secs(1),
"the budget is what ends the wait");
            Ok(())
        })
}

#[test]
fn a_settled_retry_decision_ends_the_wait() -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .start_paused(true)
        .build()?
        .block_on(async {
            let clock = TestClock::at(0);
            let cooldown = NavigationOutcome::CoolingDown(Duration::from_secs(7));
            let settled = settle(
                &clock,
                Duration::from_secs(30),
                scripted(vec![NavigationOutcome::Challenged, cooldown.clone()]),
            )
            .await?;

            check!(eq; settled, cooldown);
            Ok(())
        })
}

#[test]
fn a_budget_the_clock_cannot_hold_is_no_budget() -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .start_paused(true)
        .build()?
        .block_on(async {
            let clock = TestClock::at(0);
            let sampled = Rc::new(Cell::new(false));
            let recorder = Rc::clone(&sampled);
            let settled = settle(&clock, Duration::MAX, move || {
                recorder.set(true);
                ready(Ok(NavigationOutcome::Ready))
            })
            .await?;

            check!(eq; settled, NavigationOutcome::Challenged);
            check!(
                !sampled.get(),
                "a deadline that cannot exist must not start the wait"
            );
            Ok(())
        })
}
