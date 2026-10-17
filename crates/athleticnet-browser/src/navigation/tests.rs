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

#[test]
fn pending_headers_replay_only_on_exact_request_id() -> Result<(), Box<dyn std::error::Error>> {
    use super::observer::{is_current, navigation_status};
    use chromiumoxide::cdp::browser_protocol::network::RequestId;
    let current = Some(RequestId::new("req-1"));
    check!(
        is_current(&current, &RequestId::new("req-1")),
        "a matching request id is current"
    );
    check!(
        !is_current(&current, &RequestId::new("req-2")),
        "a stale request id never replays pending headers"
    );
    check!(
        is_current(&None, &RequestId::new("req-9")),
        "a keyless observer accepts the first arrival before a request is known"
    );
    check!(eq; navigation_status(200)?, 200);
    check!(eq; navigation_status(403)?, 403);
    check!(
        navigation_status(-1).is_err(),
        "a negative wire status is protocol, not a status"
    );
    check!(
        navigation_status(70_000).is_err(),
        "an out-of-range wire status is protocol, not a status"
    );
    Ok(())
}

mod wire;

#[test]
fn challenge_only_revokes_the_gate() -> Result<(), Box<dyn std::error::Error>> {
    use crate::clock::TestClock;
    let clock = TestClock::at(0);
    let open_gate = || {
        let gate = ProfileGate::new();
        let generation = gate.snapshot().generation;
        check!(
            gate.try_open(generation),
            "gate opens from a clean snapshot"
        );
        Ok::<_, Box<dyn std::error::Error>>(gate)
    };
    let mut limited =
        super::observer::empty_observation("https://www.athletic.net/team/123".to_owned());
    limited.status = Some(429);
    limited.body_complete = true;
    let gate = open_gate()?;
    check!(
        eq;
        classify_observation(limited, &gate, &clock)?,
        NavigationOutcome::CoolingDown(std::time::Duration::from_secs(60))
    );
    check!(
        gate.is_ready(),
        "a bare 429 cools down without revoking the gate"
    );
    let mut forbidden =
        super::observer::empty_observation("https://www.athletic.net/team/123".to_owned());
    forbidden.status = Some(403);
    forbidden.body_complete = true;
    let gate = open_gate()?;
    check!(
        eq;
        classify_observation(forbidden, &gate, &clock)?,
        NavigationOutcome::Failed(403)
    );
    check!(
        gate.is_ready(),
        "a bare 403 fails without assuming a challenge revocation"
    );
    let mut challenged =
        super::observer::empty_observation("https://www.athletic.net/team/123".to_owned());
    challenged.status = Some(200);
    challenged.body_complete = true;
    challenged.challenged = true;
    let gate = open_gate()?;
    check!(eq; classify_observation(challenged, &gate, &clock)?, NavigationOutcome::Challenged);
    check!(!gate.is_ready(), "a header challenge revokes the gate");
    let mut body =
        super::observer::empty_observation("https://www.athletic.net/team/123".to_owned());
    body.status = Some(200);
    body.body_complete = true;
    body.body_challenged = true;
    let gate = open_gate()?;
    check!(eq; classify_observation(body, &gate, &clock)?, NavigationOutcome::Challenged);
    check!(!gate.is_ready(), "a body challenge revokes the gate");
    Ok(())
}

#[test]
fn refusal_states_do_not_settle_as_ready() -> Result<(), Box<dyn std::error::Error>> {
    use crate::clock::TestClock;
    use crate::BrowserError;
    let clock = TestClock::at(0);
    let gate = ProfileGate::new();
    let mut failed =
        super::observer::empty_observation("https://www.athletic.net/team/123".to_owned());
    failed.status = Some(200);
    failed.body_complete = true;
    failed.failed = true;
    check!(matches!(
        classify_observation(failed, &gate, &clock),
        Err(BrowserError::Transport)
    ));
    let mut redirect =
        super::observer::empty_observation("https://www.athletic.net/team/123".to_owned());
    redirect.status = Some(302);
    redirect.body_complete = true;
    check!(matches!(
        classify_observation(redirect, &gate, &clock),
        Err(BrowserError::Redirect)
    ));
    let mut pending =
        super::observer::empty_observation("https://www.athletic.net/team/123".to_owned());
    pending.status = Some(200);
    pending.body_complete = false;
    check!(
        eq;
        classify_observation(pending, &gate, &clock)?,
        NavigationOutcome::Pending
    );
    let mut ready =
        super::observer::empty_observation("https://www.athletic.net/team/123".to_owned());
    ready.status = Some(200);
    ready.body_complete = true;
    check!(
        eq;
        classify_observation(ready, &gate, &clock)?,
        NavigationOutcome::Ready
    );
    Ok(())
}
