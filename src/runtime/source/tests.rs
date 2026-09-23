use super::*;
use crate::domain::evidence::Sport;
use url::Url;

#[test]
fn team_route_retains_indoor_season_identifier() {
    let origin = Url::parse("http://127.0.0.1:9090/").expect("origin");
    let resource = SourceResource::Team {
        team_id: 7,
        sport: Sport::TrackField,
        season: 12025,
    };
    let request = request::build(&origin, &resource).expect("request");
    assert_eq!(request.url.path(), "/api/v1/TeamNav/Team");
    assert!(request
        .url
        .query_pairs()
        .any(|(key, value)| key == "season" && value == "12025"));
}

#[test]
fn receiptless_transport_fault_is_retryable_for_rankings() {
    // The browser client lost the command response: no receipt exists, nothing
    // was observed, and the session re-arm makes the retry meaningful.
    assert!(receiptless_transport(Some(FailureCode::Transport), false));
}

#[test]
fn observed_ranking_faults_are_never_retried() {
    // A receipt means the source answered (403/429/challenge/parse).  Retrying
    // would hammer the source and risk discarding the retained evidence.
    assert!(!receiptless_transport(Some(FailureCode::Transport), true));
    assert!(!receiptless_transport(Some(FailureCode::HttpFailure), true));
    assert!(!receiptless_transport(
        Some(FailureCode::RateLimited),
        false
    ));
    assert!(!receiptless_transport(
        Some(FailureCode::BrowserUnavailable),
        false
    ));
    assert!(!receiptless_transport(None, false));
}
