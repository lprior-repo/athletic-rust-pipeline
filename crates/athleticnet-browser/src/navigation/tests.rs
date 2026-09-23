use super::*;
use crate::clock::TestClock;
use std::cell::Cell;
use std::future::{ready, Ready};
use std::rc::Rc;

/// A sampler that walks a script of outcomes, then repeats its last one.
fn scripted(
    script: Vec<NavigationOutcome>,
) -> impl FnMut() -> Ready<Result<NavigationOutcome, BrowserError>> {
    let mut index = 0;
    move || {
        let outcome = script
            .get(index)
            .or_else(|| script.last())
            .cloned()
            .unwrap_or(NavigationOutcome::Challenged);
        index += 1;
        ready(Ok(outcome))
    }
}

#[tokio::test(start_paused = true)]
async fn a_challenge_that_clears_itself_settles_on_the_page_it_became() {
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
    .await
    .expect("a scripted sampler never fails");

    assert_eq!(settled, NavigationOutcome::Ready);
    // Three poll intervals: the wait re-samples rather than spinning or sleeping out the
    // budget it was given.
    assert_eq!(started.elapsed(), CHALLENGE_POLL * 3);
}

#[tokio::test(start_paused = true)]
async fn a_challenge_that_outlasts_the_budget_is_reported_as_challenged() {
    let clock = TestClock::at(0);
    let started = tokio::time::Instant::now();
    let settled = settle(
        &clock,
        Duration::from_secs(1),
        scripted(vec![NavigationOutcome::Challenged]),
    )
    .await
    .expect("a scripted sampler never fails");

    assert_eq!(settled, NavigationOutcome::Challenged);
    assert_eq!(
        started.elapsed(),
        Duration::from_secs(1),
        "the budget is what ends the wait"
    );
}

#[tokio::test(start_paused = true)]
async fn a_settled_retry_decision_ends_the_wait() {
    let clock = TestClock::at(0);
    let cooldown = NavigationOutcome::CoolingDown(Duration::from_secs(7));
    let settled = settle(
        &clock,
        Duration::from_secs(30),
        scripted(vec![NavigationOutcome::Challenged, cooldown.clone()]),
    )
    .await
    .expect("a scripted sampler never fails");

    // A 429's `Retry-After` is the caller's decision, not a challenge to wait out.
    assert_eq!(settled, cooldown);
}

#[tokio::test(start_paused = true)]
async fn a_budget_the_clock_cannot_hold_is_no_budget() {
    let clock = TestClock::at(0);
    let sampled = Rc::new(Cell::new(false));
    let recorder = Rc::clone(&sampled);
    let settled = settle(&clock, Duration::MAX, move || {
        recorder.set(true);
        ready(Ok(NavigationOutcome::Ready))
    })
    .await
    .expect("a scripted sampler never fails");

    assert_eq!(settled, NavigationOutcome::Challenged);
    assert!(
        !sampled.get(),
        "a deadline that cannot exist must not start the wait"
    );
}
