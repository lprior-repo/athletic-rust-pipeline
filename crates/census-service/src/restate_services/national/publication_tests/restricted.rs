use super::NationalCensus;
use super::{fixtures, journal};
use census_store::clock::Clock;
use restate_sdk::prelude::*;
use std::sync::Arc;
use std::time::Duration;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(super) async fn exercise() -> TestResult {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let url = format!(
        "http://{}/invoke/NationalCensus/run",
        listener.local_addr()?
    );
    let directory = tempfile::tempdir()?;
    let store = Arc::new(census_store::Store::open(directory.path())?);
    let region = Arc::new(crate::spawn::Spawner::new());
    let jobs = crate::restate_services::Jobs::new(store, Arc::clone(&region));
    let clock: Arc<dyn Clock> = Arc::new(super::FixtureClock);
    let endpoint = Endpoint::builder()
        .bind(NationalCensus::new(clock, jobs))
        .build();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let server = HttpServer::new(endpoint).serve_with_cancel(listener, async move {
        match stop_rx.await {
            Ok(()) | Err(_) => {}
        }
    });
    let assertions = async {
        let result = drive_restricted(&url).await;
        stop_tx
            .send(())
            .map_err(|_| "fixture endpoint lost its shutdown owner")?;
        result
    };
    let ((), result) = tokio::join!(server, assertions);
    region.drain(Duration::from_secs(5)).await?;
    result
}

async fn drive_restricted(url: &str) -> TestResult {
    tokio::time::timeout(Duration::from_secs(30), async {
        let client = reqwest::Client::builder()
            .http2_prior_knowledge()
            .timeout(Duration::from_secs(5))
            .build()?;
        let controlled = crate::restate_services::jobs::selected_fixture::install()
            .map_err(|error| error.to_string())?;
        let request = fixtures::selected_request(
            &controlled,
            &[
                census_domain::UsJurisdiction::Michigan,
                census_domain::UsJurisdiction::Minnesota,
                census_domain::UsJurisdiction::Iowa,
                census_domain::UsJurisdiction::Illinois,
                census_domain::UsJurisdiction::Colorado,
            ],
        )?;
        let mut journal = journal::Journal::new(&request)?;
        journal.selected_workbook = Some(controlled);
        journal.drive(&client, url).await?;
        verify_restricted_jurisdictions(&journal)?;
        super::assert_published(&journal)
    })
    .await?
}

fn verify_restricted_jurisdictions(journal: &journal::Journal) -> TestResult {
    let expected = serde_json::json!(["MI", "MN", "IA", "IL", "CO"]);
    let call = journal
        .calls
        .iter()
        .find(|call| call.text(1).is_ok_and(|name| name == "Workbook"))
        .ok_or("restricted publication stage absent")?;
    let body: serde_json::Value = serde_json::from_slice(call.bytes(3)?)?;
    check!(eq; body.get("jurisdictions"), Some(&expected));
    let report = journal
        .output
        .as_ref()
        .ok_or("restricted national run did not return its report")?;
    check!(eq; report.get("jurisdictions").and_then(serde_json::Value::as_array).map(Vec::len), Some(5));
    Ok(())
}
