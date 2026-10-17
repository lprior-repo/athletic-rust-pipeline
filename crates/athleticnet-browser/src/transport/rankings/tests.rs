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
        assert_eq!(next_page_after(body.as_bytes(), 1), Some(2));
    }
}
#[cfg(test)]
mod navigation_wire {
    use super::super::capture::{uniquely_bound, WireLedger};
    use crate::protocol::RankingsCapture;
    use crate::BrowserError;
    use chromiumoxide::cdp::browser_protocol::network::RequestId;

    const ORIGIN: &str = "https://www.athletic.net";
    const NAV_URL: &str =
        "https://www.athletic.net/api/v1/tfRankings/GetNavInfo?seasonId=2026&level=4&gender=m";
    const OTHER_URL: &str = "https://www.athletic.net/api/v1/tfRankings/GetRankings";

    fn ledger_with_request(id: &str) -> WireLedger {
        let mut ledger = WireLedger::new();
        ledger.note_request(
            RequestId::new(id),
            NAV_URL.to_owned(),
            "GET".to_owned(),
            None,
            false,
            ORIGIN,
            RankingsCapture::Navigation,
        );
        ledger
    }

    #[test]
    fn a_single_matching_request_binds_uniquely_regardless_of_arrival_order() -> anyhow::Result<()>
    {
        let mut early_wire = WireLedger::new();
        early_wire.note_finished(RequestId::new("req-1"), 1_048_576.0);
        early_wire.note_request(
            RequestId::new("req-1"),
            NAV_URL.to_owned(),
            "GET".to_owned(),
            None,
            false,
            ORIGIN,
            RankingsCapture::Navigation,
        );
        let mut late_wire = WireLedger::new();
        late_wire.note_request(
            RequestId::new("req-1"),
            NAV_URL.to_owned(),
            "GET".to_owned(),
            None,
            false,
            ORIGIN,
            RankingsCapture::Navigation,
        );
        late_wire.note_finished(RequestId::new("req-1"), 1_048_576.0);
        for ledger in [&early_wire, &late_wire] {
            let candidates = ledger.candidates_for(NAV_URL, "GET");
            anyhow::ensure!(candidates.len() == 1);
            let bound = uniquely_bound(candidates, 1)?;
            anyhow::ensure!(bound.is_some());
            anyhow::ensure!(
                super::super::super::transfer::refuse_excess_wire(Some(1_048_576.0)).is_ok()
            );
        }
        Ok(())
    }

    #[test]
    fn duplicate_matching_requests_refuse_as_protocol_rather_than_picking_latest(
    ) -> anyhow::Result<()> {
        let mut ledger = ledger_with_request("req-1");
        ledger.note_request(
            RequestId::new("req-2"),
            NAV_URL.to_owned(),
            "GET".to_owned(),
            None,
            false,
            ORIGIN,
            RankingsCapture::Navigation,
        );
        let candidates = ledger.candidates_for(NAV_URL, "GET");
        anyhow::ensure!(candidates.len() == 2);
        anyhow::ensure!(matches!(
            uniquely_bound(candidates, 1),
            Err(BrowserError::Protocol)
        ));
        Ok(())
    }

    #[test]
    fn duplicate_bindings_for_one_wire_refuse_as_protocol() -> anyhow::Result<()> {
        let ledger = ledger_with_request("req-1");
        let candidates = ledger.candidates_for(NAV_URL, "GET");
        anyhow::ensure!(matches!(
            uniquely_bound(candidates, 2),
            Err(BrowserError::Protocol)
        ));
        anyhow::ensure!(matches!(
            uniquely_bound(Vec::new(), 0),
            Err(BrowserError::Protocol)
        ));
        Ok(())
    }

    #[test]
    fn missing_wire_waits_rather_than_assigning_zero() -> anyhow::Result<()> {
        let ledger = ledger_with_request("req-1");
        let absent = ledger.candidates_for(
            "https://www.athletic.net/api/v1/tfRankings/GetNavInfo?seasonId=2027&level=4&gender=m",
            "GET",
        );
        anyhow::ensure!(absent.is_empty());
        anyhow::ensure!(uniquely_bound(absent, 1)?.is_none());
        let mut unrelated = WireLedger::new();
        unrelated.note_request(
            RequestId::new("other-1"),
            OTHER_URL.to_owned(),
            "POST".to_owned(),
            Some(true),
            false,
            ORIGIN,
            RankingsCapture::Results,
        );
        unrelated.note_request(
            RequestId::new("foreign-1"),
            NAV_URL.to_owned(),
            "GET".to_owned(),
            None,
            false,
            "https://other.example",
            RankingsCapture::Navigation,
        );
        anyhow::ensure!(unrelated.candidates_for(NAV_URL, "GET").is_empty());
        Ok(())
    }

    #[test]
    fn an_excess_wire_total_is_refused_before_decode() -> anyhow::Result<()> {
        let ledger = ledger_with_request("req-1");
        let candidates = ledger.candidates_for(NAV_URL, "GET");
        let bound = uniquely_bound(candidates, 1)?.ok_or(anyhow::anyhow!("missing bound"))?;
        let _ = bound;
        anyhow::ensure!(matches!(
            super::super::super::transfer::refuse_excess_wire(Some(40_000_000.0)),
            Err(BrowserError::PayloadLimit)
        ));
        anyhow::ensure!(matches!(
            super::super::super::transfer::refuse_excess_wire(Some(f64::NAN)),
            Err(BrowserError::Protocol)
        ));
        anyhow::ensure!(matches!(
            super::super::super::transfer::refuse_excess_wire(None),
            Err(BrowserError::Protocol)
        ));
        Ok(())
    }

    #[test]
    fn a_canceled_request_does_not_create_ambiguity() -> anyhow::Result<()> {
        let mut ledger = ledger_with_request("req-1");
        ledger.note_request(
            RequestId::new("req-2"),
            NAV_URL.to_owned(),
            "GET".to_owned(),
            None,
            false,
            ORIGIN,
            RankingsCapture::Navigation,
        );
        ledger.note_failed(RequestId::new("req-2"), "net::ERR_ABORTED".to_owned(), None);
        let candidates = ledger.candidates_for(NAV_URL, "GET");
        anyhow::ensure!(candidates.len() == 1);
        anyhow::ensure!(uniquely_bound(candidates, 1)?.is_some());
        Ok(())
    }
    #[test]
    fn a_stale_malformed_wire_does_not_poison_the_bound_request() -> anyhow::Result<()> {
        let mut ledger = ledger_with_request("req-1");
        ledger.note_finished(RequestId::new("req-1"), 1_048_576.0);
        ledger.note_finished(RequestId::new("stale-1"), f64::NAN);
        let candidates = ledger.candidates_for(NAV_URL, "GET");
        anyhow::ensure!(candidates.len() == 1);
        let bound = uniquely_bound(candidates, 1)?.ok_or(anyhow::anyhow!("missing bound"))?;
        anyhow::ensure!(bound == RequestId::new("req-1"));
        anyhow::ensure!(
            super::super::super::transfer::refuse_excess_wire(Some(1_048_576.0)).is_ok()
        );
        anyhow::ensure!(matches!(
            super::super::super::transfer::refuse_excess_wire(Some(f64::NAN)),
            Err(BrowserError::Protocol)
        ));
        Ok(())
    }
    #[test]
    fn a_bound_malformed_wire_is_refused_before_decode() -> anyhow::Result<()> {
        let mut ledger = ledger_with_request("req-1");
        ledger.note_finished(RequestId::new("req-1"), f64::NEG_INFINITY);
        let candidates = ledger.candidates_for(NAV_URL, "GET");
        let bound = uniquely_bound(candidates, 1)?.ok_or(anyhow::anyhow!("missing bound"))?;
        anyhow::ensure!(bound == RequestId::new("req-1"));
        anyhow::ensure!(matches!(
            super::super::super::transfer::refuse_excess_wire(Some(f64::NEG_INFINITY)),
            Err(BrowserError::Protocol)
        ));
        anyhow::ensure!(matches!(
            super::super::super::transfer::refuse_excess_wire(None),
            Err(BrowserError::Protocol)
        ));
        Ok(())
    }
    #[test]
    fn stale_redirect_and_failure_on_another_route_do_not_poison() -> anyhow::Result<()> {
        let mut ledger = ledger_with_request("req-1");
        ledger.note_request(
            RequestId::new("other-1"),
            OTHER_URL.to_owned(),
            "POST".to_owned(),
            Some(true),
            false,
            ORIGIN,
            RankingsCapture::Results,
        );
        ledger.note_response(
            RequestId::new("other-1"),
            OTHER_URL.to_owned(),
            302,
            ORIGIN,
            RankingsCapture::Results,
        );
        ledger.note_failed(
            RequestId::new("other-1"),
            "net::ERR_FAILED".to_owned(),
            None,
        );
        let candidates = ledger.candidates_for(NAV_URL, "GET");
        anyhow::ensure!(candidates.len() == 1);
        anyhow::ensure!(uniquely_bound(candidates, 1)?.is_some());
        let other = ledger.candidates_for(OTHER_URL, "POST");
        anyhow::ensure!(other.len() == 1);
        Ok(())
    }
    #[test]
    fn method_matching_ignores_case_but_not_route() -> anyhow::Result<()> {
        let ledger = ledger_with_request("req-1");
        anyhow::ensure!(ledger.candidates_for(NAV_URL, "get").len() == 1);
        anyhow::ensure!(ledger.candidates_for(NAV_URL, "POST").is_empty());
        anyhow::ensure!(ledger.candidates_for(OTHER_URL, "GET").is_empty());
        Ok(())
    }
}

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

    static SCENARIO: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

    struct HandlerGuard(tokio::task::JoinHandle<()>);

    impl Drop for HandlerGuard {
        fn drop(&mut self) {
            self.0.abort();
        }
    }

    fn var(key: &str, fallback: &str) -> String {
        match std::env::var(key) {
            Ok(value) => value,
            Err(_) => fallback.to_owned(),
        }
    }

    fn origin() -> TestResult<url::Url> {
        Ok(url::Url::parse(&var(
            "ADLAW_LANE_FIXTURE",
            "http://127.0.0.1:21045/",
        ))?)
    }

    fn client() -> reqwest::Client {
        reqwest::Client::new()
    }

    async fn state(fixture: &url::Url) -> TestResult<Value> {
        Ok(client()
            .get(fixture.join("state")?)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    async fn set_scenario(fixture: &url::Url, name: &str) -> TestResult {
        client()
            .post(fixture.join("scenario")?)
            .json(&serde_json::json!({ "scenario": name }))
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    fn api_calls(value: &Value) -> TestResult<Vec<&Value>> {
        Ok(value["requests"]
            .as_array()
            .ok_or("fixture state is missing its request ledger")?
            .iter()
            .filter(|row| row["path"].as_str() == Some(API_PATH))
            .collect())
    }

    async fn parked_page(fixture: &url::Url) -> TestResult<(Browser, Page, HandlerGuard)> {
        let cdp = var("ADLAW_LANE_CDP", "http://127.0.0.1:9223");
        let (browser, mut handler) = Browser::connect_with_config(
            cdp.as_str(),
            HandlerConfig {
                request_timeout: Duration::from_secs(20),
                ..Default::default()
            },
        )
        .await?;
        let handler = HandlerGuard(tokio::spawn(async move {
            for _ in 0..4096 {
                if handler.next().await.is_none() {
                    break;
                }
            }
        }));
        let page = browser.new_page(fixture.as_str()).await?;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
        while tokio::time::Instant::now() < deadline {
            if page
                .evaluate("location.origin")
                .await
                .ok()
                .and_then(|value| value.into_value::<String>().ok())
                .is_some_and(|value| value == fixture.origin().ascii_serialization())
            {
                return Ok((browser, page, handler));
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        Err("fixture page never reached its origin before the deadline".into())
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

    #[test]
    #[ignore = "requires a fixture origin and a CDP browser"]
    fn results_capture_costs_one_physical_post() -> TestResult {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                let _scenario = SCENARIO.lock().await;
                let fixture = origin()?;
                set_scenario(&fixture, "normal").await?;
                let (_browser, page, _handler) = parked_page(&fixture).await?;
                let gate = open_gate();
                let before = api_calls(&state(&fixture).await?)?.len();
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
                .map_err(|error| error.to_string())?;
                let ledger = state(&fixture).await?;
                let observed = api_calls(&ledger)?;
                check!(eq;
                    observed.len().checked_sub(before),
                    Some(1),
                    "exactly one physical request"
                );
                let last = observed.last().ok_or("missing physical rankings POST")?;
                check!(eq; last["method"], "POST");
                check!(eq; last["status"], 200);
                check!(eq; response.status.as_u16(), 200);
                check!(gate.is_ready(), "a served response leaves the gate open");
                let observation = response
                    .rankings
                    .ok_or("missing rankings capture metadata")?;
                check!(eq; observation.capture, RankingsCapture::Results);
                check!(eq; observation.request_method, "POST");
                check!(eq;
                    observation.request_url,
                    fixture.join(API_PATH.trim_start_matches('/'))?.as_str()
                );
                check!(eq; observation.request_body.as_deref(), Some(MEASURED_BODY));
                check!(eq; observation.next_page, Some(2));
                let envelope: Value = serde_json::from_slice(&response.body)?;
                check!(eq; envelope["settings"]["page"], 1);
                page.close().await?;
                Ok(())
            })
    }

    #[test]
    #[ignore = "requires a fixture origin and a CDP browser"]
    fn challenge_response_revokes_the_gate_and_ends_pagination() -> TestResult {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                let _scenario = SCENARIO.lock().await;
                let fixture = origin()?;
                set_scenario(&fixture, "challenge").await?;
                let (_browser, page, _handler) = parked_page(&fixture).await?;
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
                .map_err(|error| error.to_string())?;
                check!(eq; response.status.as_u16(), 403);
                check!(!gate.is_ready(), "a challenge closes the gate");
                check!(eq;
                    response
                        .rankings
                        .ok_or("missing challenge capture metadata")?
                        .next_page,
                    None,
                    "a challenged page never advances pagination"
                );
                page.close().await?;
                Ok(())
            })
    }
}
