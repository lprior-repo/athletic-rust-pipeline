//! The lane on the object's shared fetcher: what the deployment handed in is what the plan reads.
//!
//! The plan asks the fetcher rather than a flag kept beside it, so these two tests are the contract
//! between the shell's wiring and the run's source plan: an object built with a lane produces a
//! fetcher that a browser-transported source is swept through, and one built without produces the
//! fetcher that refuses that source by name.

use std::sync::Arc;

use census_crawl::net::bridge::BrowserLane;
use census_store::clock::SystemClock;
use census_store::Store;

use super::JurisdictionCensus;
use crate::ingress;
use crate::restate_services::jurisdiction::DISPATCHED;
use crate::restate_services::meets_arms::{arm_for as meets_arm_for, MEETS_ARMS};
use crate::restate_services::plan::BrowserLaneState;
use crate::restate_services::teams_arms::{arm_for, TEAMS_ARMS};

/// The chain's dispatched slugs and the stage arm tables are one list told twice: the plan promises
/// a stage runs every unit it calls sweepable, and the arms are what keeps that promise. A slug
/// added to one list and not the other is either a refusal nothing explains or a walk nothing runs,
/// so the union of the tables is held equal to the dispatched list here rather than reconciled at
/// run time.
#[test]
fn the_arms_are_the_dispatched_slugs() {
    use crate::restate_services::results_arms::RESULTS_ARMS;

    let mut arms: Vec<&str> = TEAMS_ARMS.iter().map(|(slug, _)| *slug).collect();
    arms.extend(MEETS_ARMS.iter().map(|(slug, _)| *slug));
    arms.extend(RESULTS_ARMS.iter().map(|(slug, _)| *slug));
    assert_eq!(arms, DISPATCHED);
    assert!(
        arm_for("no-stage-runs-this").is_none(),
        "an unknown slug has no arm, and the stage turns that into a terminal error"
    );
    assert!(
        meets_arm_for("no-stage-runs-this").is_none(),
        "an unknown slug has no arm, and the stage turns that into a terminal error"
    );
}

/// The two arm tables are disjoint: a planned unit both stages armed would run twice under one plan
/// entry, and the second walk's rows would be attributed to a source the first already wrote.
#[test]
fn no_slug_is_armed_by_two_stages() {
    for (slug, _) in TEAMS_ARMS {
        assert!(
            meets_arm_for(slug).is_none(),
            "{slug} is armed by the team-index stage and the meet-index stage"
        );
    }
}

/// A store the object can build its shared fetcher over. No network is touched: the fetcher's cache
/// directory is the only thing construction reads.
fn object(lane: Option<BrowserLane>) -> (tempfile::TempDir, JurisdictionCensus) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Arc::new(Store::open(dir.path()).expect("store"));
    let census = JurisdictionCensus::new(store, Arc::new(SystemClock), lane);
    (dir, census)
}

/// The deployment serves the profile: the object's fetcher carries the client, and the plan
/// classifies a browser-transported source as ordinary work.
#[tokio::test]
async fn the_fetcher_carries_the_lane_the_endpoint_was_given() {
    let client = ingress::client(ingress::DEFAULT_ORIGIN).expect("the deployment's ingress origin");
    let (_dir, census) = object(Some(BrowserLane::over(client)));

    let fetcher = census.fetcher().await.expect("the fetcher builds");
    assert_eq!(BrowserLaneState::of(&fetcher), BrowserLaneState::Configured);
}

/// The control: no lane handed in means no lane installed, which is what makes the plan refuse that
/// source instead of spending attempts on it.
#[tokio::test]
async fn without_a_lane_the_fetcher_has_none_and_the_plan_refuses() {
    let (_dir, census) = object(None);

    let fetcher = census.fetcher().await.expect("the fetcher builds");
    assert_eq!(BrowserLaneState::of(&fetcher), BrowserLaneState::Absent);
}
