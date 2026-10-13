use athleticnet_browser::{BrowserCapture, BrowserResponse};
use restate_sdk::http_server::HttpServer;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use super::*;

fn field(key: u8, value: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut encoded = vec![key];
    let mut length = u64::try_from(value.len())?;
    while length >= 128 {
        encoded.push(u8::try_from(length & 0x7f)? | 0x80);
        length >>= 7;
    }
    encoded.push(u8::try_from(length)?);
    encoded.extend_from_slice(value);
    Ok(encoded)
}

fn frame(kind: u16, value: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut encoded = kind.to_be_bytes().to_vec();
    encoded.extend_from_slice(&[0, 0]);
    encoded.extend_from_slice(&u32::try_from(value.len())?.to_be_bytes());
    encoded.extend_from_slice(value);
    Ok(encoded)
}

fn recorded_capture() -> BrowserOutcome {
    BrowserOutcome::Captured(Box::new(BrowserCapture {
        response: BrowserResponse {
            status: reqwest::StatusCode::OK,
            response_url: Some("https://www.athletic.net/track-and-field/rankings".to_string()),
            headers: reqwest::header::HeaderMap::new(),
            body: b"completed browser capture".to_vec(),
            rankings: None,
        },
        challenge: false,
        retry_after_ms: None,
        fetched_at_ms: Some(1_795_000_000_000),
    }))
}

fn replay_journal(outcome: &BrowserOutcome, completed: bool) -> anyhow::Result<Vec<u8>> {
    let request = RequestSpec {
        url: Url::parse("https://www.athletic.net/track-and-field/rankings")?,
        semantic_url: "https://www.athletic.net/track-and-field/rankings".to_string(),
        action: RequestAction::Fetch {},
        headers: Vec::new(),
    };
    let mut start = field(0x0a, b"browser-fetch-replay")?;
    start.extend(field(0x12, b"browser-fetch-replay")?);
    start.extend_from_slice(&[0x18, if completed { 4 } else { 3 }]);
    start.extend(field(0x32, SESSION_KEY.as_bytes())?);
    let input = field(0x72, &field(0x0a, &serde_json::to_vec(&request)?)?)?;
    let result = field(0x0a, &serde_json::to_vec(outcome)?)?;
    let mut notification = vec![0x08, 1];
    notification.extend(field(0x2a, &result)?);
    let mut journal = frame(0, &start)?;
    journal.extend(frame(0x0400, &input)?);
    journal.extend(frame(0x0411, &[0x58, 1])?);
    journal.extend(frame(0x8011, &notification)?);
    if completed {
        journal.extend(frame(0x0401, &field(0x72, &result)?)?);
    }
    Ok(journal)
}

async fn invoke_replay(journal: Vec<u8>) -> anyhow::Result<Vec<u8>> {
    let session = session()?;
    let retained = session.clone();
    let server = HttpServer::new(Endpoint::builder().bind(session).build());
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let client = reqwest::Client::builder()
        .http2_prior_knowledge()
        .timeout(Duration::from_secs(5))
        .build()?;
    let (shutdown, stopped) = oneshot::channel::<()>();
    let request = async move {
        let result = async {
            let response = client
                .post(format!("http://{address}/invoke/BrowserSession/fetch"))
                .header("content-type", "application/vnd.restate.invocation.v5")
                .body(journal)
                .send()
                .await?;
            anyhow::ensure!(response.status() == reqwest::StatusCode::OK);
            Ok::<_, anyhow::Error>(response.bytes().await?.to_vec())
        }
        .await;
        shutdown
            .send(())
            .map_err(|_| anyhow::anyhow!("the SDK server stopped before shutdown"))?;
        result
    };
    let (_, result) =
        futures::future::join(server.serve_with_cancel(listener, stopped), request).await;
    anyhow::ensure!(
        retained.lane.lock().await.manager.is_none(),
        "replay installed a browser manager"
    );
    anyhow::ensure!(
        retained.lane.lock().await.epoch == 0,
        "replay changed the launch epoch"
    );
    anyhow::ensure!(
        retained.admission.try_lock().is_ok(),
        "replay left a browser launch admitted"
    );
    result
}

#[tokio::test]
async fn a_recorded_fetch_result_replays_without_a_live_manager() -> anyhow::Result<()> {
    let outcome = recorded_capture();
    let response = invoke_replay(replay_journal(&outcome, false)?).await?;
    let value = field(0x0a, &serde_json::to_vec(&outcome)?)?;
    let mut expected = frame(0x0401, &field(0x72, &value)?)?;
    expected.extend(frame(3, &[])?);
    anyhow::ensure!(
        response == expected,
        "recorded capture replay differs: {response:?}"
    );
    Ok(())
}

#[tokio::test]
async fn a_completed_fetch_journal_is_consumed_without_a_live_manager() -> anyhow::Result<()> {
    let response = invoke_replay(replay_journal(&recorded_capture(), true)?).await?;
    anyhow::ensure!(
        response == frame(3, &[])?,
        "completed journal replay did not end cleanly: {response:?}"
    );
    Ok(())
}
