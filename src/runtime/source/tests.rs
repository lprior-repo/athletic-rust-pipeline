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
fn valid_retry_after_overrides_fallback() {
    let attempt = http::AttemptResult {
        receipt: None,
        code: Some(FailureCode::RateLimited),
        status: Some(429),
        message: String::new(),
        retryable: true,
        retry_after_ms: 1_000,
    };
    assert_eq!(
        retry::next_delay(0, &attempt, Duration::from_secs(90)),
        Ok(Duration::from_secs(1))
    );
}

#[test]
fn absent_retry_after_uses_bounded_rate_limit_fallback() {
    let attempt = http::AttemptResult {
        receipt: None,
        code: Some(FailureCode::RateLimited),
        status: Some(429),
        message: String::new(),
        retryable: true,
        retry_after_ms: 0,
    };
    assert_eq!(
        retry::next_delay(1, &attempt, Duration::from_secs(1)),
        Ok(Duration::from_secs(120))
    );
}
