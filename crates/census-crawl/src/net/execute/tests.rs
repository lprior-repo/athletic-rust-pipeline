//! Pause-time tests for the fetch loop's timer: the per-host pacing.
//!
//! It runs on the clock capability, so a paused clock makes the test deterministic: the assertions
//! are about the virtual-time deltas themselves rather than "it eventually returned", and nothing
//! here sleeps in real time. The pacing is driven through the same functions the fetch loop calls —
//! [`Fetcher::host_gate`] and [`Fetcher::wait_turn`] — so what is asserted is the loop's own timing,
//! not a model of it.
//!
//! Retry timing is deliberately absent: the durable layer owns retries (ADR-002), so there is no
//! in-process backoff schedule left to assert.
//!
//! No test here performs IO: with a paused clock, a real socket wait parks the runtime and the
//! clock auto-advances to the next timer (the client's request timeout), which is real-time
//! dependent and therefore not an assertion that could be deterministic.

use super::*;
use crate::net::Fetcher;
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
    fetcher.host_gate(HOST, Some(Duration::from_secs(3))).await;

    let start = tokio::time::Instant::now();
    fetcher.wait_turn(HOST).await;
    assert_eq!(
        tokio::time::Instant::now().duration_since(start),
        Duration::ZERO
    );

    fetcher.wait_turn(HOST).await;
    assert_eq!(
        tokio::time::Instant::now().duration_since(start),
        Duration::from_secs(3),
        "the second turn must leave exactly the robots crawl-delay after the first"
    );

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

    let mut turn = Box::pin(fetcher.wait_turn(HOST));
    assert!(
        tokio::time::timeout(Duration::ZERO, &mut turn)
            .await
            .is_err(),
        "the turn passed the gate without waiting for the host's spacing"
    );

    tokio::time::advance(Duration::from_millis(2999)).await;
    assert!(
        tokio::time::timeout(Duration::ZERO, &mut turn)
            .await
            .is_err(),
        "a partial advance opened the gate early"
    );

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

    assert_eq!(condition.id, format!("forbidden:{HOST}"));
    assert_eq!(condition.source, "milesplit");
    assert_eq!(condition.status, 403);

    let now = crate::net::now_iso8601();
    assert_eq!(fetcher.access_conditions().await.len(), 1);
    assert!(fetcher.host_blocked(HOST, &now).await);
    assert_eq!(fetcher.blocked_hosts(&now).await, vec![HOST.to_string()]);

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

    let start = tokio::time::Instant::now();
    fetcher.wait_turn("ca.milesplit.com").await;
    assert_eq!(
        tokio::time::Instant::now().duration_since(start),
        Duration::from_secs(2)
    );
}

#[tokio::test(start_paused = true)]
async fn a_host_outside_every_family_keeps_its_own_budget() {
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
