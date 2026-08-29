use crate::alpha_api::{AlphaApiError, AlphaApiClientConfig};
use crate::alpha_api_client::AlphaApiClient;
use crate::alpha_model::{AlphaRequest, PaginationConfig};

fn make_config(url: &str, routes: &[&str], fields: Vec<&str>) -> AlphaApiClientConfig {
    AlphaApiClientConfig {
        base_url: url.to_owned(), rankings_path: "/rankings".to_owned(),
        nav_info_path: "/nav".to_owned(), timeout_seconds: 30, max_retries: 2,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".into() },
        allowed_routes: routes.iter().map(|s| (*s).to_string()).collect(),
        allowed_fields: fields.iter().map(|s| s.to_string()).collect(),
        max_concurrent_requests: 1, min_delay_ms: 0, max_retry_delay_ms: 30_000, cap_markers: vec![],
    }
}

fn make_test_req() -> AlphaRequest {
    AlphaRequest { state_id: 1, season_id: 2024, gender: "Female".into(),
        event_short: "100m".into(), indoor: false, continuation: None }
}

#[tokio::test(flavor = "multi_thread")]
async fn nextpage_cap_marker_rejected() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let s = mockito::Server::new();
        let u = s.url();
        (s, u)
    }).await.unwrap();
    server.mock("POST", "/rankings")
        .match_body(mockito::Matcher::Any)
        .with_status(200).with_header("content-type", "application/json")
        .with_body(r#"{"page":1,"complete":false,"continuation":null,
        "groupedRankings":[[{"AthleteID":1,"AthleteName":"Test",
        "GradeID":1,"TeamName":"","State":""}]],"hasMore":true,
        "nextPage":"next-token","__cap":true}"#).create();
    let client = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: url, rankings_path: "/rankings".into(), nav_info_path: "/nav".into(),
        timeout_seconds: 30, max_retries: 2,
        pagination: PaginationConfig::NextPage { has_more_pointer: "/hasMore".into(),
            next_page_pointer: "/nextPage".into(), request_page_key: "page".into() },
        allowed_routes: vec!["/rankings".into()],
        allowed_fields: vec!["AthleteID".into(),"AthleteName".into(),"GradeID".into(),
            "TeamName".into(),"State".into(),"MeetID".into(),"MeetName".into(),
            "IDResult".into(),"EventShort".into(),"Measure".into(),
            "ResultDate".into(),"SeasonID".into()],
        max_concurrent_requests: 1, min_delay_ms: 0, max_retry_delay_ms: 30_000, cap_markers: vec!["__cap".into()],
    }).expect("client must not fail");
    let err = client.rankings(&make_test_req()).await.unwrap_err();
    assert!(matches!(err, AlphaApiError::TruncatedWithoutContinuation));
}

#[tokio::test(flavor = "multi_thread")]
async fn single_response_cap_marker_rejected() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let s = mockito::Server::new();
        let u = s.url();
        (s, u)
    }).await.unwrap();
    server.mock("POST", "/api")
        .match_body(mockito::Matcher::Any)
        .with_status(200).with_header("content-type", "application/json")
        .with_body(r#"{"page":1,"complete":false,"continuation":null,
        "groupedRankings":[[{"AthleteID":1,"AthleteName":"Test",
        "GradeID":1,"TeamName":"","State":""}]],"__cap":true}"#).create();
    let client = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: url, rankings_path: "/api".into(), nav_info_path: "/nav".into(),
        timeout_seconds: 30, max_retries: 2,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".into() },
        allowed_routes: vec!["/api".into()],
        allowed_fields: vec!["AthleteID".into(),"AthleteName".into(),"GradeID".into(),
            "TeamName".into(),"State".into(),"MeetID".into(),"MeetName".into(),
            "IDResult".into(),"EventShort".into(),"Measure".into(),
            "ResultDate".into(),"SeasonID".into()],
        max_concurrent_requests: 1, min_delay_ms: 0, max_retry_delay_ms: 30_000, cap_markers: vec!["__cap".into()],
    }).expect("client must not fail");
    let err = client.rankings(&make_test_req()).await.unwrap_err();
    assert!(matches!(err, AlphaApiError::TruncatedWithoutContinuation));
}

#[test]
fn enforce_allowed_fields_removes_nested_results_disallowed() {
    let client = AlphaApiClient::new(make_config("http://example.com",
        &["/api"], vec!["AthleteID","AthleteName","GradeID","TeamName","State",
        "MeetID","MeetName","IDResult","EventShort","Measure","ResultDate","SeasonID"]))
        .expect("client must not fail");
    let value: serde_json::Value = serde_json::from_str(r#"{
        "page":1,"complete":false,"groupedRankings":[[{
            "AthleteID":1,"AthleteName":"Test","GradeID":1,"TeamName":"","State":"",
            "Results":[{"IDResult":100,"EventShort":"100m","Measure":"10.50",
            "ResultDate":"2024-01-01","SeasonID":2024,"MeetID":500,"MeetName":"Meet",
            "Wind":"+0.5"}]}]]}"#).unwrap();
    let filtered = client.enforce_response_allowed_fields(value).unwrap();
    assert!(filtered.get("page").is_some());
    assert!(filtered.get("complete").is_some());
    let recs = filtered["groupedRankings"][0].as_array().unwrap();
    let results = recs[0]["Results"].as_array().unwrap();
    let r = &results[0];
    assert!(r.get("IDResult").is_some());
    assert!(r.get("EventShort").is_some());
    assert!(r.get("Measure").is_some());
    assert!(r.get("ResultDate").is_some());
    assert!(r.get("SeasonID").is_some());
    assert!(r.get("MeetID").is_some());
    assert!(r.get("MeetName").is_some());
    assert!(r.get("Wind").is_none());
}

#[test]
fn enforce_allowed_fields_preserves_envelope() {
    let client = AlphaApiClient::new(make_config("http://example.com",
        &["/api"], vec!["AthleteID","AthleteName","GradeID","TeamName","State",
        "MeetID","MeetName","IDResult","EventShort","Measure","ResultDate","SeasonID"]))
        .expect("client must not fail");
    let value: serde_json::Value = serde_json::from_str(r#"{
        "page":1,"complete":false,"continuation":"token",
        "groupedRankings":[[{"AthleteID":1,"AthleteName":"Test",
        "GradeID":1,"TeamName":"","State":""}]]}"#).unwrap();
    let filtered = client.enforce_response_allowed_fields(value).unwrap();
    assert!(filtered.get("page").is_some());
    assert!(filtered.get("complete").is_some());
    assert!(filtered.get("continuation").is_some());
    assert!(filtered.get("groupedRankings").is_some());
}

#[test]
fn enforce_missing_required_result_field_errors() {
    let client = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: "http://example.com".into(), rankings_path: "/api".into(),
        nav_info_path: "/nav".into(), timeout_seconds: 10, max_retries: 0,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".into() },
        allowed_routes: vec!["/api".into()],
        allowed_fields: vec!["AthleteID".into(), "AthleteName".into(), "GradeID".into(),
            "TeamName".into(), "State".into(), "MeetID".into(), "IDResult".into(),
            "EventShort".into(), "Measure".into(), "ResultDate".into(), "SeasonID".into()],
        max_concurrent_requests: 1, min_delay_ms: 0, max_retry_delay_ms: 30_000, cap_markers: vec![],
    }).expect("client must not fail");
    let err = client.enforce_response_allowed_fields(serde_json::json!({})).unwrap_err();
    assert!(matches!(err, AlphaApiError::Incomplete(msg) if msg.contains("MeetName")));
}

#[test]
fn enforce_full_allowed_fields_passes() {
    let client = AlphaApiClient::new(make_config("http://example.com",
        &["/api"], vec!["AthleteID","AthleteName","GradeID","TeamName","State",
        "MeetID","MeetName","IDResult","EventShort","Measure","ResultDate","SeasonID"]))
        .expect("client must not fail");
    let value: serde_json::Value = serde_json::from_str(r#"{
        "page":1,"complete":false,"groupedRankings":[[{
            "AthleteID":1,"AthleteName":"Test","GradeID":1,"TeamName":"",
            "State":"","MeetID":50,"MeetName":"Regional",
            "Results":[{"IDResult":100,"EventShort":"100m","Measure":"10.50",
            "ResultDate":"2024-01-01","SeasonID":2024,"Wind":"+0.5"}]}]]}"#).unwrap();
    let filtered = client.enforce_response_allowed_fields(value).unwrap();
    let rec = &filtered["groupedRankings"][0][0];
    assert!(rec.get("AthleteID").is_some());
    assert!(rec.get("AthleteName").is_some());
    assert!(rec.get("GradeID").is_some());
    assert!(rec.get("TeamName").is_some());
    assert!(rec.get("State").is_some());
    assert!(rec.get("MeetID").is_some());
    assert!(rec.get("MeetName").is_some());
    assert!(rec.get("Wind").is_none());
    let r = &rec["Results"][0];
    assert!(r.get("IDResult").is_some());
    assert!(r.get("EventShort").is_some());
    assert!(r.get("Measure").is_some());
    assert!(r.get("ResultDate").is_some());
    assert!(r.get("SeasonID").is_some());
    assert!(r.get("Wind").is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn enforce_response_allowed_fields_removes_wind_unknown_keeps_envelope() {
    // Finding 5: disallowed Wind and unknown fields absent;
    // required fields and envelope pointers remain.
    let client = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: "http://localhost".into(),
        rankings_path: "/api/v1/tfRankings/GetRankings".into(),
        nav_info_path: "/api/v1/tfRankings/GetNavInfo".into(),
        timeout_seconds: 30, max_retries: 2,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".into() },
        allowed_routes: vec!["/api/v1/tfRankings/GetRankings".into()],
        allowed_fields: vec!["AthleteID".into(), "AthleteName".into(), "GradeID".into(),
            "TeamName".into(), "State".into(), "MeetID".into(), "MeetName".into(),
            "IDResult".into(), "EventShort".into(), "Measure".into(),
            "ResultDate".into(), "SeasonID".into()],
        max_concurrent_requests: 1, min_delay_ms: 0, max_retry_delay_ms: 30_000, cap_markers: vec![],
    }).unwrap();
    let value: serde_json::Value = serde_json::from_str(r#"{
        "groupedRankings":[[{"AthleteID":1,"AthleteName":"Test","GradeID":2,
            "TeamName":"School","State":"CA",
            "Results":[{"MeetID":100,"MeetName":"State Finals",
            "IDResult":500,"EventShort":"100m","Measure":"10.55",
            "ResultDate":"2026-06-15","SeasonID":2026,"Wind":"+0.5",
            "UnknownField":"should be removed"}]}]],
        "page":1,"complete":true,"continuation":null}"#).unwrap();
    let filtered = client.enforce_response_allowed_fields(value).unwrap();
    assert!(filtered.get("groupedRankings").is_some());
    assert!(filtered.get("page").is_some());
    assert!(filtered.get("complete").is_some());
    assert!(filtered.get("continuation").is_some());
    assert!(filtered["groupedRankings"][0][0].get("Wind").is_none());
    assert!(filtered["groupedRankings"][0][0]["Results"][0].get("Wind").is_none());
    assert!(filtered["groupedRankings"][0][0]["Results"][0].get("UnknownField").is_none());
    let rec = &filtered["groupedRankings"][0][0];
    assert!(rec.get("AthleteID").is_some());
    assert!(rec.get("AthleteName").is_some());
    assert!(rec.get("GradeID").is_some());
    assert!(rec.get("TeamName").is_some());
    assert!(rec.get("State").is_some());
    let r = &rec["Results"][0];
    assert!(r.get("MeetID").is_some());
    assert!(r.get("MeetName").is_some());
    assert!(r.get("IDResult").is_some());
    assert!(r.get("EventShort").is_some());
    assert!(r.get("Measure").is_some());
    assert!(r.get("ResultDate").is_some());
    assert!(r.get("SeasonID").is_some());
}
