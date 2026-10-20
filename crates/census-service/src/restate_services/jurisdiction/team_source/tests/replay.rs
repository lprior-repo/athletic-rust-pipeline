use super::{key, ledger, request};
use crate::restate_services::jurisdiction::JurisdictionCensus;
use crate::restate_services::wire::{TeamsSourceOutcome, TeamsSourceRequest};
use crate::restate_services::{JobError, Jobs};
use census_reconcile::identity::WorkflowIdentity;
use census_store::{clock::SystemClock, Store};
use restate_sdk::prelude::*;
use std::sync::Arc;
use std::time::Duration;

use crate::restate_services::national::publication_tests::codec;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[tokio::test]
async fn replay_consumes_registered_runs_when_live_teams_ledger_has_settled() -> TestResult {
    exercise("run").await
}

#[tokio::test]
async fn replay_consumes_registered_runs_when_live_contacts_ledger_has_settled() -> TestResult {
    exercise("contacts").await
}

async fn exercise(handler: &str) -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Arc::new(Store::open(directory.path())?);
    let region = Arc::new(crate::spawn::Spawner::new());
    let owner = JurisdictionCensus::new(
        Arc::clone(&store),
        Arc::new(SystemClock),
        None,
        crate::restate_services::CaptureExecution::LegacyFresh,
        Jobs::new(Arc::clone(&store), Arc::clone(&region)),
    );
    let source =
        super::super::TeamsSource::new(owner, Jobs::new(Arc::clone(&store), Arc::clone(&region)));
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let url = format!(
        "http://{}/invoke/TeamsSource/{handler}",
        listener.local_addr()?
    );
    let endpoint = Endpoint::builder().bind(source).build();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let server = HttpServer::new(endpoint).serve_with_cancel(listener, async move {
        match stop_rx.await {
            Ok(()) | Err(_) => {}
        }
    });
    let assertions = async {
        let result =
            tokio::time::timeout(Duration::from_secs(30), replay(&store, &url, handler)).await;
        stop_tx
            .send(())
            .map_err(|_| "fixture endpoint lost shutdown owner")?;
        result?
    };
    let ((), result) = tokio::join!(server, assertions);
    region.drain(Duration::from_secs(5)).await?;
    result
}

async fn replay(store: &Store, url: &str, handler: &str) -> TestResult {
    let client = reqwest::Client::builder()
        .http2_prior_knowledge()
        .timeout(Duration::from_secs(5))
        .build()?;
    let mut request = request()?;
    let operation = if handler == "contacts" {
        request.source = "sidearm_staff".to_owned();
        let jurisdiction = &request.jurisdiction;
        let identity = WorkflowIdentity::jurisdiction(
            jurisdiction.jurisdiction,
            jurisdiction.season,
            jurisdiction.revision,
        );
        super::super::contacts_key(identity.as_str())
    } else {
        key(&request)
    };
    let mut journal = vec![codec::Frame::new(
        0x0400,
        codec::blob(14, &codec::value(&serde_json::to_vec(&request)?)?)?,
    )];
    record_registration(&client, url, &operation, &mut journal).await?;
    check!(eq; ledger::status(store, &operation, Some(&request))?.map(|_| "settled"), None);
    let registered = ledger::register(store, &operation, &request)?;
    let ledger::AdmissionReplay::Reserved { attempt, .. } = ledger::begin_at(
        store,
        &operation,
        &request,
        &registered,
        ledger::baseline(store, &operation)?,
    )?
    else {
        return Err("fixture settled before its physical attempt".into());
    };
    let ledger::Completion::Settled(expected) = ledger::finish(
        store,
        &operation,
        attempt,
        ledger::Attempt::Failed(JobError::Terminal {
            message: "source access refused".to_owned(),
        }),
        &[],
    )?
    else {
        return Err("fixture failure did not settle".into());
    };
    let before = store.journal_keys(ledger::PHASE)?;
    let mut frames = invoke(&client, url, &operation, &journal).await?;
    for _ in 0..2 {
        if frames.iter().any(|frame| frame.kind == 0x0401) {
            break;
        }
        for frame in frames {
            match frame.kind {
                0x0411 => journal.push(frame),
                0x0005 => journal.push(codec::notification(
                    0x8011,
                    frame.number(1)?,
                    frame.bytes(14)?,
                )?),
                0x0001 | 0x0003 => {}
                _ => return Err(format!("unexpected settlement replay frame {frame:?}").into()),
            }
        }
        frames = invoke(&client, url, &operation, &journal).await?;
    }
    check!(eq; output(&frames)?, serde_json::to_value(&expected)?);
    check!(!frames
        .iter()
        .any(|frame| frame.kind == 0x0411 || frame.kind == 0x0005));
    check!(eq; store.journal_keys(ledger::PHASE)?, before);
    check!(!store.journal_contains(ledger::PHASE, &format!("{operation}/attempt/2/reserved"))?);
    println!("replayed {handler}: consumed {} recorded Runs with settled live ledger; no second reservation", journal.iter().filter(|frame| frame.kind == 0x0411).count());
    reattach(&client, url, &operation, &request, &expected).await?;
    check!(eq; store.journal_keys(ledger::PHASE)?, before);
    Ok(())
}

async fn record_registration(
    client: &reqwest::Client,
    url: &str,
    operation: &str,
    journal: &mut Vec<codec::Frame>,
) -> TestResult {
    for _ in 0..3 {
        let frames = invoke(client, url, operation, journal).await?;
        let mut registered = false;
        for frame in frames {
            match frame.kind {
                0x0411 => journal.push(frame),
                0x0005 => {
                    let value: serde_json::Value = serde_json::from_slice(frame.bytes(14)?)?;
                    registered |= value.get("request_digest").is_some();
                    journal.push(codec::notification(
                        0x8011,
                        frame.number(1)?,
                        frame.bytes(14)?,
                    )?);
                }
                0x0001 | 0x0003 => {}
                _ => return Err(format!("unexpected registration frame {frame:?}").into()),
            }
        }
        if registered {
            return Ok(());
        }
    }
    Err("registration Run was not recorded within three resumptions".into())
}

async fn reattach(
    client: &reqwest::Client,
    url: &str,
    operation: &str,
    request: &TeamsSourceRequest,
    expected: &TeamsSourceOutcome,
) -> TestResult {
    let mut journal = vec![codec::Frame::new(
        0x0400,
        codec::blob(14, &codec::value(&serde_json::to_vec(request)?)?)?,
    )];
    let frames = invoke(client, url, operation, &journal).await?;
    let runs: Vec<_> = frames.iter().filter(|frame| frame.kind == 0x0411).collect();
    check!(eq; runs.len(), 1);
    for frame in frames {
        match frame.kind {
            0x0411 => journal.push(frame),
            0x0005 => journal.push(codec::notification(
                0x8011,
                frame.number(1)?,
                frame.bytes(14)?,
            )?),
            0x0001 | 0x0003 => {}
            _ => return Err(format!("unexpected reattach frame {frame:?}").into()),
        }
    }
    check!(eq; output(&invoke(client, url, operation, &journal).await?)?, serde_json::to_value(expected)?);
    Ok(())
}

pub(super) async fn invoke(
    client: &reqwest::Client,
    url: &str,
    operation: &str,
    journal: &[codec::Frame],
) -> TestResult<Vec<codec::Frame>> {
    let mut start = codec::blob(1, b"teams-source-replay-fixture")?;
    start.extend(codec::blob(2, b"teams-source-replay-fixture")?);
    start.extend(codec::integer(3, u64::try_from(journal.len())?));
    start.extend(codec::blob(6, operation.as_bytes())?);
    let mut body = codec::Frame::new(0, start).encode()?;
    for frame in journal {
        body.extend(frame.encode()?);
    }
    let response = client
        .post(url)
        .header("content-type", "application/vnd.restate.invocation.v6")
        .body(body)
        .send()
        .await?;
    check!(eq; response.status(), reqwest::StatusCode::OK);
    codec::frames(&response.bytes().await?)
}

fn output(frames: &[codec::Frame]) -> TestResult<serde_json::Value> {
    if let Some(error) = frames.iter().find(|frame| frame.kind == 0x0002) {
        return Err(format!("SDK replay failed: {}", error.text(2)?).into());
    }
    let output = frames
        .iter()
        .find(|frame| frame.kind == 0x0401)
        .ok_or("missing handler output")?;
    let value = codec::field(output.bytes(14)?, 1)?.ok_or("missing output value")?;
    Ok(serde_json::from_slice(value)?)
}
