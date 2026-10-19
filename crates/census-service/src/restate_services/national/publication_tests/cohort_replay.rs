use super::{codec, journal, transport, FixtureClock, NationalCensus, TestResult};
use crate::restate_services::wire::NationalRequest;
use restate_sdk::prelude::*;
use std::{sync::Arc, time::Duration};

fn original_prefix() -> TestResult<journal::Journal> {
    let metadata: serde_json::Value =
        serde_json::from_str(include_str!("original_d6_bind_prefix.json"))?;
    let mut frames = Vec::new();
    for entry in metadata["frames"].as_array().ok_or("original frames")? {
        let kind = match entry["entry_type"].as_str().ok_or("original entry type")? {
            "Command: Input" => 0x0400,
            "Command: Call" => 0x040D,
            "Command: SetState" => 0x0403,
            "Notification: CallInvocationId" => 0x800E,
            "Notification: Call" => 0x800D,
            other => return Err(format!("unsupported original entry {other}").into()),
        };
        let hex = entry["raw_hex"].as_str().ok_or("original protobuf")?;
        let mut pairs = hex.as_bytes().chunks_exact(2);
        let payload = pairs
            .by_ref()
            .map(|pair| Ok(u8::from_str_radix(std::str::from_utf8(pair)?, 16)?))
            .collect::<TestResult<Vec<_>>>()?;
        if !pairs.remainder().is_empty() {
            return Err("truncated original hexadecimal byte".into());
        }
        frames.push(codec::Frame::new(kind, payload));
    }
    let input = frames.first().ok_or("original Input")?;
    let request: NationalRequest =
        serde_json::from_slice(codec::field(input.bytes(14)?, 1)?.ok_or("original Input value")?)?;
    let mut replay = journal::Journal::new(&request)?;
    replay.entries = frames;
    replay.invocation = metadata["invocation"]
        .as_str()
        .ok_or("original invocation")?
        .to_owned();
    replay.identity = metadata["workflow_key"]
        .as_str()
        .ok_or("original workflow key")?
        .to_owned();
    Ok(replay)
}

async fn replay_once(url: &str) -> TestResult {
    let replay = original_prefix()?;
    let expected = super::super::targets(&replay.original).map_err(|error| format!("{error:?}"))?;
    let client = reqwest::Client::builder()
        .http2_prior_knowledge()
        .timeout(Duration::from_secs(10))
        .build()?;
    let output = transport::exchange(&client, url, replay.request_body()?, &mut None).await?;
    transport::terminal(&output)?;
    if let Some(error) = output.iter().find(|frame| frame.kind == 0x0002) {
        let mut detail = error.text(2)?;
        if let Ok(description) = error.text(3) {
            detail.push_str(&description);
        }
        return Err(format!(
            "original native replay failed ({}): {detail}",
            error.number(1)?
        )
        .into());
    }
    let calls: Vec<_> = output.iter().filter(|frame| frame.kind == 0x040D).collect();
    check!(eq; calls.len(), expected.len());
    for (_, key) in expected {
        let call = calls
            .iter()
            .find(|frame| frame.text(5).is_ok_and(|actual| actual == key))
            .ok_or_else(|| format!("original jurisdiction not dispatched: {key}"))?;
        check!(eq; call.text(1)?, "JurisdictionCensus");
        check!(eq; call.text(2)?, "run");
    }
    check!(!output.iter().any(|frame| frame.kind == 0x0403));
    Ok(())
}

#[tokio::test]
async fn original_d6_bind_prefix_replays_through_first_jurisdiction_dispatch() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Arc::new(census_store::Store::open(directory.path())?);
    let region = Arc::new(crate::spawn::Spawner::new());
    let jobs = crate::restate_services::Jobs::new(Arc::clone(&store), Arc::clone(&region));
    let endpoint = Endpoint::builder()
        .bind(NationalCensus::new(Arc::new(FixtureClock), jobs))
        .build();
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let url = format!(
        "http://{}/invoke/NationalCensus/run",
        listener.local_addr()?
    );
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let server = HttpServer::new(endpoint).serve_with_cancel(listener, async move {
        match stop_rx.await {
            Ok(()) | Err(_) => {}
        }
    });
    let assertions = async move {
        let result = replay_once(&url).await;
        stop_tx
            .send(())
            .map_err(|_| "replay endpoint lost shutdown owner")?;
        result
    };
    let ((), result) = tokio::join!(server, assertions);
    region.drain(Duration::from_secs(5)).await?;
    result?;
    check!(store.run_manifest()?.is_none());
    Ok(())
}
