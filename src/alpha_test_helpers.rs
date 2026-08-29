use crate::alpha_api::AlphaApiClientConfig;
use crate::alpha_api_client::AlphaApiClient;
use crate::alpha_model::{AlphaRequest, PaginationConfig};

pub fn make_client(url: &str) -> AlphaApiClient {
    AlphaApiClient::new(AlphaApiClientConfig {
        base_url: url.to_owned(),
        rankings_path: "/api/v1/tfRankings/GetRankings".into(),
        nav_info_path: "/api/v1/tfRankings/GetNavInfo".into(),
        timeout_seconds: 30,
        max_retries: 2,
        pagination: PaginationConfig::SingleResponse {
            complete_pointer: "/complete".into(),
        },
        allowed_routes: vec![
            "/api/v1/tfRankings/GetRankings".into(),
            "/api/v1/tfRankings/GetNavInfo".into(),
        ],
        allowed_fields: vec![
            "AthleteID".into(), "AthleteName".into(), "GradeID".into(),
            "TeamName".into(), "State".into(),
            "MeetID".into(), "MeetName".into(),
            "IDResult".into(), "EventShort".into(), "Measure".into(),
            "ResultDate".into(), "SeasonID".into(),
        ],
        max_concurrent_requests: 1,
        min_delay_ms: 0,
        cap_markers: vec![],
    })
    .expect("client creation must not fail")
}

pub fn make_test_request() -> AlphaRequest {
    AlphaRequest {
        state_id: 12,
        season_id: 2026,
        gender: "m".into(),
        event_short: "100m".into(),
        indoor: false,
        continuation: None,
    }
}

pub fn success_body() -> &'static str {
    r#"{"groupedRankings":[[{"AthleteID":1,"AthleteName":"Test","GradeID":2,"TeamName":"School","State":"CA","Results":[{"MeetID":100,"MeetName":"State Finals","IDResult":500,"EventShort":"100m","Measure":"10.55","ResultDate":"2026-06-15","SeasonID":2026,"Wind":null}]}]],"page":1,"complete":true,"continuation":null}"#
}

pub fn make_full_pagination_config(url: &str) -> AlphaApiClient {
    AlphaApiClient::new(AlphaApiClientConfig {
        base_url: url.to_owned(),
        rankings_path: "/api/v1/tfRankings/GetRankings".into(),
        nav_info_path: "/api/v1/tfRankings/GetNavInfo".into(),
        timeout_seconds: 30,
        max_retries: 2,
        pagination: PaginationConfig::NextPage {
            request_page_key: "page".into(),
            has_more_pointer: "/hasMore".into(),
            next_page_pointer: "/nextPage".into(),
        },
        allowed_routes: vec![
            "/api/v1/tfRankings/GetRankings".into(),
            "/api/v1/tfRankings/GetNavInfo".into(),
        ],
        allowed_fields: vec![
            "AthleteID".into(), "AthleteName".into(), "GradeID".into(),
            "TeamName".into(), "State".into(),
            "MeetID".into(), "MeetName".into(),
            "IDResult".into(), "EventShort".into(), "Measure".into(),
            "ResultDate".into(), "SeasonID".into(),
        ],
        max_concurrent_requests: 1,
        min_delay_ms: 0,
        cap_markers: vec![],
    })
    .expect("client creation must not fail")
}

pub fn make_client_with_fields(url: &str, allowed_fields: Vec<&str>) -> AlphaApiClient {
    AlphaApiClient::new(AlphaApiClientConfig {
        base_url: url.to_owned(),
        rankings_path: "/api".into(),
        nav_info_path: "/nav".into(),
        timeout_seconds: 10,
        max_retries: 0,
        pagination: PaginationConfig::SingleResponse {
            complete_pointer: "/complete".into(),
        },
        allowed_routes: vec!["/api".into()],
        allowed_fields: allowed_fields.iter().map(|s| s.to_string()).collect(),
        max_concurrent_requests: 1,
        min_delay_ms: 0,
        cap_markers: vec![],
    })
    .expect("client must not fail")
}
