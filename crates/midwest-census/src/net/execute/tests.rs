//! Pause-time tests for the fetch loop's two timers: the per-host pacing and the retry backoff.
//!
//! Both run on the clock capability, so a paused clock makes them deterministic: the assertions are
//! about the virtual-time deltas themselves rather than "it eventually returned", and nothing here
//! sleeps in real time. The pacing and the backoff are driven through the same functions the fetch
//! loop calls — [`Fetcher::host_gate`] and [`Fetcher::wait_turn`] for the spacing,
//! [`crate::net::request::wait_backoff`] for the retry — so what is asserted is the loop's own
//! timing, not a model of it.
//!
//! No test here performs IO: with a paused clock, a real socket wait parks the runtime and the
//! clock auto-advances to the next timer (the client's request timeout), which is real-time
//! dependent and therefore not an assertion that could be deterministic.

use super::*;
use crate::net::request::wait_backoff;
use crate::net::{jittered_delay, Fetcher};
use std::collections::HashMap;
use std::time::Duration;

/// A host name for the pacing map. Nothing resolves it: these tests never open a socket.
const HOST: &str = "www.example.test";

/// A fetcher whose cache is a throwaway directory.
fn fetcher_in(dir: &std::path::Path, delay: Duration, authorized: Vec<String>) -> Fetcher {
    Fetcher::new(dir.join("http"), None, delay, HashMap::new(), authorized).expect("fetcher")
}

#[tokio::test(start_paused = true)]
async fn turns_for_one_host_are_spaced_by_exactly_the_robots_delay() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fetcher = fetcher_in(dir.path(), Duration::from_millis(1), Vec::new());
    // The delay an origin asks for in robots.txt becomes the host's spacing: the configured
    // millisecond does not lower it.
    fetcher.host_gate(HOST, Some(Duration::from_secs(3))).await;

    let start = tokio::time::Instant::now();
    // Nothing is reserved for the host yet, so the first turn waits for nothing.
    fetcher.wait_turn(HOST).await;
    assert_eq!(
        tokio::time::Instant::now().duration_since(start),
        Duration::ZERO
    );

    // The first turn reserved a slot one delay ahead, so the second leaves exactly that delay.
    fetcher.wait_turn(HOST).await;
    assert_eq!(
        tokio::time::Instant::now().duration_since(start),
        Duration::from_secs(3),
        "the second turn must leave exactly the robots crawl-delay after the first"
    );

    // Each turn pushes the slot one delay further instead of resetting it, so the spacing holds for
    // a queue of turns rather than only for the first pair.
    fetcher.wait_turn(HOST).await;
    assert_eq!(
        tokio::time::Instant::now().duration_since(start),
        Duration::from_secs(6),
        "the third turn must resume from the pushed slot, not recompute it"
    );
}

#[tokio::test(start_paused = true)]
async fn a_partial_advance_leaves_the_next_turn_gated() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fetcher = fetcher_in(dir.path(), Duration::from_millis(1), Vec::new());
    fetcher.host_gate(HOST, Some(Duration::from_secs(3))).await;
    fetcher.wait_turn(HOST).await;
    let reserved_at = tokio::time::Instant::now();

    // The next turn is polled rather than awaited, so the assertions can look at the gate from
    // inside its wait.
    let mut turn = Box::pin(fetcher.wait_turn(HOST));
    assert!(
        tokio::time::timeout(Duration::ZERO, &mut turn)
            .await
            .is_err(),
        "the turn passed the gate without waiting for the host's spacing"
    );

    // One millisecond short of the reserved slot: still gated.
    tokio::time::advance(Duration::from_millis(2999)).await;
    assert!(
        tokio::time::timeout(Duration::ZERO, &mut turn)
            .await
            .is_err(),
        "a partial advance opened the gate early"
    );

    // The last millisecond of the spacing: the gate opens.
    tokio::time::advance(Duration::from_millis(1)).await;
    turn.await;
    assert_eq!(
        tokio::time::Instant::now().duration_since(reserved_at),
        Duration::from_secs(3)
    );
}

#[tokio::test(start_paused = true)]
async fn an_authorized_host_is_never_paced_faster_than_the_policy_ceiling() {
    let dir = tempfile::tempdir().expect("temp dir");
    // The operator authorized the host and configured a millisecond of spacing: authorization
    // records that robots rules are logged rather than enforced, never that the 2 rps policy
    // ceiling is relaxed.
    let fetcher = fetcher_in(
        dir.path(),
        Duration::from_millis(1),
        vec!["example.test".to_string()],
    );
    fetcher
        .host_gate(HOST, Some(Duration::from_millis(1)))
        .await;

    let start = tokio::time::Instant::now();
    fetcher.wait_turn(HOST).await;
    fetcher.wait_turn(HOST).await;
    assert_eq!(
        tokio::time::Instant::now().duration_since(start),
        MIN_AUTHORIZED_DELAY,
        "an authorized host's spacing must stay at the policy ceiling"
    );
}

#[tokio::test(start_paused = true)]
async fn a_retry_backoff_advances_by_exactly_the_schedule() {
    let dir = tempfile::tempdir().expect("temp dir");
    let _fetcher = fetcher_in(dir.path(), Duration::from_millis(1), Vec::new());
    let start = tokio::time::Instant::now();
    let mut scheduled = Duration::ZERO;

    for attempt in 1..=3 {
        let waited_from = tokio::time::Instant::now();
        let waited = wait_backoff(attempt).await;
        assert_eq!(
            waited,
            jittered_delay(attempt),
            "attempt {attempt} must wait the schedule's delay"
        );
        assert_eq!(
            tokio::time::Instant::now().duration_since(waited_from),
            waited,
            "attempt {attempt} must consume exactly the delay it waited"
        );
        scheduled += waited;
        assert_eq!(
            tokio::time::Instant::now().duration_since(start),
            scheduled,
            "attempt {attempt} must resume from the previous backoff, not restart it"
        );
    }

    // The schedule this measured is the documented one: 500 ms doubling per attempt, ±25% jitter.
    // Without that, an exact delta would only prove that a sleep slept.
    for (attempt, low, high) in [(1_u32, 375_u64, 625_u64), (2, 750, 1250), (3, 1500, 2500)] {
        let delay = jittered_delay(attempt).as_millis();
        assert!(
            delay >= u128::from(low) && delay <= u128::from(high),
            "attempt {attempt} left its band: {delay} ms not in {low}..={high} ms"
        );
    }
}

#[tokio::test(start_paused = true)]
async fn one_blocking_status_mints_one_condition_for_the_host() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fetcher =
        fetcher_in(dir.path(), Duration::from_millis(1), Vec::new()).with_source("milesplit");
    let condition = fetcher
        .record_access_condition(
            HOST,
            AccessBlockKind::Forbidden,
            403,
            None,
            "GET /teams/1/roster",
        )
        .await;

    // The row is what a lane stops on, so it has to name the host, the source and the status.
    assert_eq!(condition.id, format!("forbidden:{HOST}"));
    assert_eq!(condition.source, "milesplit");
    assert_eq!(condition.status, 403);

    let now = crate::net::now_iso8601();
    assert_eq!(fetcher.access_conditions().await.len(), 1);
    assert!(fetcher.host_blocked(HOST, &now).await);
    assert_eq!(fetcher.blocked_hosts(&now).await, vec![HOST.to_string()]);

    // A second observation of the same block refreshes the row: 582 refusals from one host are one
    // finding, not 582.
    fetcher
        .record_access_condition(HOST, AccessBlockKind::Forbidden, 403, None, "again")
        .await;
    assert_eq!(fetcher.access_conditions().await.len(), 1);
}

#[tokio::test(start_paused = true)]
async fn a_retry_after_becomes_the_cooldown_and_blocks_only_its_own_host() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fetcher = fetcher_in(dir.path(), Duration::from_millis(1), Vec::new());
    let condition = fetcher
        .record_access_condition(HOST, AccessBlockKind::RateLimited, 429, Some(120), "GET /x")
        .await;
    assert_eq!(condition.retry_after_seconds, Some(120));

    let now = crate::net::now_iso8601();
    assert_eq!(fetcher.blocked_hosts(&now).await, vec![HOST.to_string()]);
    assert!(
        !fetcher.host_blocked("other.example.test", &now).await,
        "a condition blocks the host that stated it and no other"
    );

    // Past its cooldown the condition stops blocking: the row stays as evidence, the block lifts.
    let after = condition.cooldown_until.clone().expect("cooldown");
    assert!(!condition.is_blocking(&after));
    assert!(fetcher.blocked_hosts(&after).await.is_empty());
}

/// A fetcher whose hosts share one budget per configured source family.
fn fetcher_with_families(
    dir: &std::path::Path,
    delay: Duration,
    families: &[(&str, Duration)],
) -> Fetcher {
    let families = families
        .iter()
        .map(|(family, delay)| (family.to_string(), *delay))
        .collect();
    Fetcher::new(dir.join("http"), None, delay, HashMap::new(), Vec::new())
        .expect("fetcher")
        .with_family_budgets(families)
}

#[tokio::test(start_paused = true)]
async fn two_hosts_of_one_family_share_one_budget() {
    // The measured refusal: the national walk holds one budget per state subdomain, MileSplit
    // enforces one for the client, so 51 states spend 51 budgets against one enforced one. Charging
    // both hosts to the family is the fix, and the proof is that the second host waits for the
    // first host's slot instead of pacing on its own.
    let dir = tempfile::tempdir().expect("temp dir");
    let fetcher = fetcher_with_families(
        dir.path(),
        Duration::from_millis(1),
        &[("milesplit.com", Duration::from_secs(2))],
    );
    fetcher.host_gate("tx.milesplit.com", None).await;
    fetcher.host_gate("wi.milesplit.com", None).await;

    let start = tokio::time::Instant::now();
    fetcher.wait_turn("tx.milesplit.com").await;
    fetcher.wait_turn("wi.milesplit.com").await;
    assert_eq!(
        tokio::time::Instant::now().duration_since(start),
        Duration::from_secs(2),
        "the family's spacing must hold across its hosts, not per host"
    );

    // A third host of the same family continues the family's clock rather than restarting it.
    let start = tokio::time::Instant::now();
    fetcher.wait_turn("ca.milesplit.com").await;
    assert_eq!(
        tokio::time::Instant::now().duration_since(start),
        Duration::from_secs(2)
    );
}

#[tokio::test(start_paused = true)]
async fn a_host_outside_every_family_keeps_its_own_budget() {
    // The family entry is an override, not an extra layer: a source that enforces its ceiling per
    // host must not be slowed to the family rate, and a family must not be slowed by an unrelated
    // host's traffic. Each budget's spacing is therefore measured on its own first turn pair, and
    // the other budget's clock is shown not to have moved.
    let dir = tempfile::tempdir().expect("temp dir");
    let fetcher = fetcher_with_families(
        dir.path(),
        Duration::from_millis(1),
        &[("milesplit.com", Duration::from_secs(2))],
    );
    fetcher.host_gate("tx.milesplit.com", None).await;
    fetcher.host_gate("opentrack.test", None).await;

    let start = tokio::time::Instant::now();
    fetcher.wait_turn("opentrack.test").await;
    fetcher.wait_turn("opentrack.test").await;
    let host_only = tokio::time::Instant::now().duration_since(start);
    assert_eq!(
        host_only,
        Duration::from_millis(1),
        "the host paces on its own configured spacing, not the family's"
    );

    fetcher.wait_turn("tx.milesplit.com").await;
    assert_eq!(
        tokio::time::Instant::now().duration_since(start),
        host_only,
        "the family's first turn is not delayed by another budget's traffic"
    );
    fetcher.wait_turn("tx.milesplit.com").await;
    assert_eq!(
        tokio::time::Instant::now().duration_since(start),
        host_only + Duration::from_secs(2),
        "and the family still spaces its own turns by two seconds"
    );
}

#[tokio::test(start_paused = true)]
async fn the_longest_family_key_wins() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fetcher = fetcher_with_families(
        dir.path(),
        Duration::from_millis(1),
        &[
            ("milesplit.com", Duration::from_secs(2)),
            ("slow.milesplit.com", Duration::from_secs(5)),
        ],
    );
    assert_eq!(
        fetcher.family_of("slow.milesplit.com").as_deref(),
        Some("slow.milesplit.com")
    );
    assert_eq!(
        fetcher.family_of("tx.milesplit.com").as_deref(),
        Some("milesplit.com")
    );
    assert_eq!(
        fetcher.family_of("milesplit.com").as_deref(),
        Some("milesplit.com")
    );
    assert_eq!(fetcher.family_of("notmilesplit.com"), None);
    assert_eq!(fetcher.family_of("example.test"), None);
}
