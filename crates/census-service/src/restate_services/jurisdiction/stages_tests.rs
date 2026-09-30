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

#[test]
fn no_slug_is_armed_by_two_stages() {
    for (slug, _) in TEAMS_ARMS {
        assert!(
            meets_arm_for(slug).is_none(),
            "{slug} is armed by the team-index stage and the meet-index stage"
        );
    }
}

fn object(lane: Option<BrowserLane>) -> (tempfile::TempDir, JurisdictionCensus) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Arc::new(Store::open(dir.path()).expect("store"));
    let census = JurisdictionCensus::new(store, Arc::new(SystemClock), lane);
    (dir, census)
}

#[tokio::test]
async fn the_fetcher_carries_the_lane_the_endpoint_was_given() {
    let client = ingress::client(ingress::DEFAULT_ORIGIN).expect("the deployment's ingress origin");
    let (_dir, census) = object(Some(BrowserLane::over(client)));

    let fetcher = census
        .fetcher(&[], census_crawl::net::DEFAULT_FAMILY_PARALLELISM)
        .await
        .expect("the fetcher builds");
    assert_eq!(BrowserLaneState::of(&fetcher), BrowserLaneState::Configured);
}

#[tokio::test]
async fn without_a_lane_the_fetcher_has_none_and_the_plan_refuses() {
    let (_dir, census) = object(None);

    let fetcher = census
        .fetcher(&[], census_crawl::net::DEFAULT_FAMILY_PARALLELISM)
        .await
        .expect("the fetcher builds");
    assert_eq!(BrowserLaneState::of(&fetcher), BrowserLaneState::Absent);
}
