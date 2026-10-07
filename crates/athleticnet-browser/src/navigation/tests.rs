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
#[test]
fn an_admitted_document_is_only_the_target_origin() -> Result<(), Box<dyn std::error::Error>> {
    let target = url::Url::parse("https://www.athletic.net/team/123")?;
    check!(
        !foreign_final_document("https://www.athletic.net/team/456", &target),
        "a same-origin document URL is admitted"
    );
    check!(
        foreign_final_document("https://other.example/foreign", &target),
        "a foreign-origin document URL is refused"
    );
    check!(
        foreign_final_document("http://www.athletic.net/team/123", &target),
        "a scheme downgrade is a different origin and is refused"
    );
    check!(
        foreign_final_document("not-a-url", &target),
        "an unparseable document URL is refused"
    );
    Ok(())
}

#[test]
fn a_foreign_final_document_never_opens_the_profile_gate() -> Result<(), Box<dyn std::error::Error>>
{
    let admitted = url::Url::parse("https://www.athletic.net/team/123")?;
    let gate = ProfileGate::new();
    check!(
        eq;
        refuse_foreign_document("https://other.example/landing", &admitted, &gate),
        Some(NavigationOutcome::Failed(404)),
        "a document the source did not serve fails instead of settling"
    );
    check!(
        !gate.snapshot().ready,
        "a foreign final document leaves the profile gate closed"
    );
    check!(
        refuse_foreign_document(admitted.as_str(), &admitted, &gate).is_none(),
        "the admitted origin proceeds to classification"
    );
    Ok(())
}
