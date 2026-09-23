#[cfg(test)]
mod pagination {
    use super::super::results::next_page_after;

    #[test]
    fn a_page_with_rows_requests_its_successor() {
        let body = br#"{"groupedRankings":[[{"athleteId":1}]]}"#;
        assert_eq!(next_page_after(body, 1), Some(2));
        assert_eq!(next_page_after(body, 7), Some(8));
    }

    #[test]
    fn an_empty_page_terminates_the_chain() {
        assert_eq!(next_page_after(br#"{"groupedRankings":[]}"#, 3), None);
        assert_eq!(next_page_after(br#"{"groupedRankings":[[]]}"#, 3), None);
    }

    #[test]
    fn an_unparsable_page_terminates_rather_than_advancing() {
        assert_eq!(next_page_after(b"<html>challenge</html>", 3), None);
        assert_eq!(next_page_after(b"", 3), None);
    }

    #[test]
    fn a_page_below_its_declared_depth_ends_the_chain() {
        // Live shape: a complete event list shorter than the declared depth.
        let mut rows = String::new();
        for rank in 1..=71 {
            if rank > 1 {
                rows.push(',');
            }
            rows.push_str(&format!("{{\"athleteId\":{rank}}}"));
        }
        let body =
            format!("{{\"settings\":{{\"depth\":100,\"page\":1}},\"groupedRankings\":[[{rows}]]}}");
        assert_eq!(next_page_after(body.as_bytes(), 1), None);
    }

    #[test]
    fn a_page_filling_its_declared_depth_requests_its_successor() {
        let mut rows = String::new();
        for rank in 1..=100 {
            if rank > 1 {
                rows.push(',');
            }
            rows.push_str(&format!("{{\"athleteId\":{rank}}}"));
        }
        let body = format!("{{\"settings\":{{\"depth\":100}},\"groupedRankings\":[[{rows}]]}}");
        assert_eq!(next_page_after(body.as_bytes(), 1), Some(2));
    }

    #[test]
    fn default_settings_supply_the_depth_when_settings_are_absent() {
        let short = br#"{"defaultSettings":{"depth":2},"groupedRankings":[[{"a":1}]]}"#;
        assert_eq!(next_page_after(short, 1), None);
        let full = br#"{"defaultSettings":{"depth":2},"groupedRankings":[[{"a":1},{"a":2}]]}"#;
        assert_eq!(next_page_after(full, 1), Some(2));
    }

    #[test]
    fn a_deep_request_answered_with_the_listing_head_ends_the_chain() {
        // Live shape: the final page filled the declared depth and requested its
        // successor, and the source answered that request with the listing's
        // first page carrying the full row count plus the blurred tail.
        let mut rows = String::new();
        for rank in 1..=101 {
            if rank > 1 {
                rows.push(',');
            }
            rows.push_str(&format!("{{\"athleteId\":{rank}}}"));
        }
        let body =
            format!("{{\"settings\":{{\"depth\":100,\"page\":1}},\"groupedRankings\":[[{rows}]]}}");
        assert_eq!(next_page_after(body.as_bytes(), 246), None);
        // The listing head itself still requests its successor.
        assert_eq!(next_page_after(body.as_bytes(), 1), Some(2));
    }
}
/// End-to-end qualification of the persistent lane against an offline fixture.
///
/// Ignored by default: each test needs a fixture origin serving the rankings API
/// and a CDP browser. Point `ADLAW_LANE_FIXTURE` (default `http://127.0.0.1:21045/`)
/// and `ADLAW_LANE_CDP` (default `http://127.0.0.1:9223`) at those, then run
/// `cargo test --lib -- --ignored lane_smoke`.
#[cfg(test)]
mod lane_smoke {
    use super::super::session::fetch_rankings;
    use crate::gate::ProfileGate;
    use crate::protocol::RankingsCapture;
    use crate::request::RankingsAction;
    use chromiumoxide::{handler::HandlerConfig, Browser, Page};
    use futures::StreamExt;
    use serde_json::Value;
    use std::sync::Arc;
    use std::time::Duration;

    const API_PATH: &str = "/api/v1/tfRankings/GetRankings";
    const MEASURED_BODY: &str = r#"{"reportType":"div","mode":"list","divListId":168416,"indoor":null,"eventShort":"100m","gender":"m","qParams":{"grades":[11],"page":1},"qualifyingListKey":"","version":2,"debug":""}"#;

    /// The fixture holds one global scenario, so a test that sets a scenario
    /// must not overlap another test running against the same origin.
    static SCENARIO: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    fn var(key: &str, fallback: &str) -> String {
        std::env::var(key).unwrap_or_else(|_| fallback.to_owned())
    }

    fn origin() -> url::Url {
        url::Url::parse(&var("ADLAW_LANE_FIXTURE", "http://127.0.0.1:21045/"))
            .expect("fixture origin")
    }

    fn client() -> reqwest::Client {
        reqwest::Client::new()
    }

    async fn state(fixture: &url::Url) -> Value {
        client()
            .get(fixture.join("state").expect("state url"))
            .send()
            .await
            .expect("fixture state")
            .json()
            .await
            .expect("fixture state json")
    }

    async fn set_scenario(fixture: &url::Url, name: &str) {
        client()
            .post(fixture.join("scenario").expect("scenario url"))
            .json(&serde_json::json!({ "scenario": name }))
            .send()
            .await
            .expect("fixture scenario");
    }

    fn api_calls(value: &Value) -> Vec<Value> {
        value["requests"]
            .as_array()
            .map(|rows| {
                rows.iter()
                    .filter(|row| row["path"].as_str() == Some(API_PATH))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Connect a browser whose first page is parked on the fixture origin: the
    /// lane issues its request from that document, so the page must be there.
    async fn parked_page(fixture: &url::Url) -> (Browser, Page) {
        let cdp = var("ADLAW_LANE_CDP", "http://127.0.0.1:9223");
        let (browser, mut handler) = Browser::connect_with_config(
            cdp.as_str(),
            HandlerConfig {
                request_timeout: Duration::from_secs(20),
                ..Default::default()
            },
        )
        .await
        .expect("connect browser");
        tokio::spawn(async move { while handler.next().await.is_some() {} });
        let page = browser
            .new_page(fixture.as_str())
            .await
            .expect("open fixture page");
        let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
        while tokio::time::Instant::now() < deadline {
            if page
                .evaluate("location.origin")
                .await
                .ok()
                .and_then(|value| value.into_value::<String>().ok())
                .is_some_and(|value| value == fixture.origin().ascii_serialization())
            {
                return (browser, page);
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("fixture page never reached its origin");
    }

    fn results_action(page: u32) -> RankingsAction {
        RankingsAction {
            list_id: 168416,
            gender: "m".to_owned(),
            grade: Some(11),
            event_short: "100m".to_owned(),
            page,
            capture: RankingsCapture::Results,
        }
    }

    fn open_gate() -> Arc<ProfileGate> {
        let gate = Arc::new(ProfileGate::new());
        let generation = gate.snapshot().generation;
        assert!(
            gate.try_open(generation),
            "gate opens from a clean snapshot"
        );
        gate
    }

    #[tokio::test]
    #[ignore = "requires a fixture origin and a CDP browser"]
    async fn results_capture_costs_one_physical_post() {
        let _scenario = SCENARIO.lock().await;
        let fixture = origin();
        set_scenario(&fixture, "normal").await;
        let (_browser, page) = parked_page(&fixture).await;
        let gate = open_gate();
        let before = api_calls(&state(&fixture).await).len();
        let response = fetch_rankings(
            &page,
            &results_action(1),
            Duration::from_secs(20),
            gate.clone(),
            &fixture,
            1,
            &crate::clock::SystemClock,
        )
        .await
        .expect("lane response");
        let observed = api_calls(&state(&fixture).await);
        assert_eq!(observed.len() - before, 1, "exactly one physical request");
        assert_eq!(observed[observed.len() - 1]["method"], "POST");
        assert_eq!(observed[observed.len() - 1]["status"], 200);
        assert_eq!(response.status.as_u16(), 200);
        assert!(gate.is_ready(), "a served response leaves the gate open");
        let observation = response.rankings.expect("capture metadata");
        assert_eq!(observation.capture, RankingsCapture::Results);
        assert_eq!(observation.request_method, "POST");
        assert_eq!(
            observation.request_url,
            fixture
                .join(API_PATH.trim_start_matches('/'))
                .expect("api url")
                .as_str()
        );
        assert_eq!(observation.request_body.as_deref(), Some(MEASURED_BODY));
        assert_eq!(observation.next_page, Some(2));
        let envelope: Value = serde_json::from_slice(&response.body).expect("rankings envelope");
        assert_eq!(envelope["settings"]["page"], 1);
        assert!(envelope["groupedRankings"]
            .as_array()
            .is_some_and(|groups| !groups.is_empty()));
        page.close().await.expect("close fixture page");
    }

    #[tokio::test]
    #[ignore = "requires a fixture origin and a CDP browser"]
    async fn challenge_response_revokes_the_gate_and_ends_pagination() {
        let _scenario = SCENARIO.lock().await;
        let fixture = origin();
        set_scenario(&fixture, "challenge").await;
        let (_browser, page) = parked_page(&fixture).await;
        let gate = open_gate();
        let response = fetch_rankings(
            &page,
            &results_action(1),
            Duration::from_secs(20),
            gate.clone(),
            &fixture,
            1,
            &crate::clock::SystemClock,
        )
        .await
        .expect("lane response");
        assert_eq!(response.status.as_u16(), 403);
        assert!(!gate.is_ready(), "a challenge closes the gate");
        assert_eq!(
            response.rankings.expect("capture metadata").next_page,
            None,
            "a challenged page never advances pagination"
        );
        page.close().await.expect("close fixture page");
    }
}
