use super::*;
use crate::restate_services::national::publication_tests::codec::{self, Frame};

const RUN_COMPLETION_ID: u64 = 2;

#[path = "request_wire.rs"]
mod request_wire;

pub(super) async fn serve<F, Fut>(fixture: &Fixture, exercise: F) -> TestResult
where
    F: FnOnce(reqwest::Client, String) -> Fut,
    Fut: std::future::Future<Output = TestResult>,
{
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let url = format!(
        "http://{}/invoke/ResultsStageBridge/run",
        listener.local_addr()?
    );
    let endpoint = Endpoint::builder().bind(fixture.bridge.clone()).build();
    let client = reqwest::Client::builder()
        .http2_prior_knowledge()
        .timeout(WAIT)
        .build()?;
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let server = HttpServer::new(endpoint).serve_with_cancel(listener, async move {
        match stopped.await {
            Ok(()) | Err(_) => {}
        }
    });
    let assertions = async move {
        let result = tokio::time::timeout(Duration::from_secs(20), exercise(client, url)).await;
        stop.send(())
            .map_err(|()| "fixture endpoint lost shutdown owner")?;
        result?
    };
    let ((), result) = tokio::join!(server, assertions);
    result
}

pub(super) fn input(fixture: &Fixture) -> TestResult<Vec<Frame>> {
    Ok(vec![Frame::new(
        0x0400,
        codec::blob(14, &codec::value(&serde_json::to_vec(&fixture.request)?)?)?,
    )])
}

pub(super) fn recorded(fixture: &Fixture, outcome: &ResultsStageOutcome) -> TestResult<Vec<Frame>> {
    let mut journal = input(fixture)?;
    journal.push(retained_state(fixture)?);
    journal.push(Frame::new(0x0411, codec::integer(11, RUN_COMPLETION_ID)));
    journal.push(codec::notification(
        0x8011,
        RUN_COMPLETION_ID,
        &serde_json::to_vec(outcome)?,
    )?);
    Ok(journal)
}

pub(super) fn recorded_failure(fixture: &Fixture) -> TestResult<Vec<Frame>> {
    let mut journal = input(fixture)?;
    journal.push(retained_state(fixture)?);
    journal.push(Frame::new(0x0411, codec::integer(11, RUN_COMPLETION_ID)));
    let mut failure = codec::integer(1, 409);
    failure.extend(codec::blob(2, b"recorded capture owner conflict")?);
    let mut completion = codec::integer(1, RUN_COMPLETION_ID);
    completion.extend(codec::blob(6, &failure)?);
    journal.push(Frame::new(0x8011, completion));
    Ok(journal)
}

fn retained_state(fixture: &Fixture) -> TestResult<Frame> {
    let mut state = codec::blob(1, crate::restate_services::KEY_STATE.as_bytes())?;
    state.extend(codec::blob(
        14,
        &codec::value(&serde_json::to_vec(&fixture.state)?)?,
    )?);
    Ok(Frame::new(0x0407, state))
}

pub(super) async fn invoke(
    client: &reqwest::Client,
    url: &str,
    fixture: &Fixture,
    journal: &[Frame],
) -> TestResult<Vec<Frame>> {
    let frames = invoke_frames(client, url, fixture, journal).await?;
    if let Some(error) = frames.iter().find(|frame| frame.kind == 0x0002) {
        return Err(format!("Results SDK invocation failed: {}", error.text(2)?).into());
    }
    Ok(frames)
}

pub(super) async fn invoke_frames(
    client: &reqwest::Client,
    url: &str,
    fixture: &Fixture,
    journal: &[Frame],
) -> TestResult<Vec<Frame>> {
    let mut start = codec::blob(1, b"results-capture-stage-replay-fixture")?;
    start.extend(codec::blob(2, b"results-capture-stage-replay-fixture")?);
    start.extend(codec::integer(3, u64::try_from(journal.len())?));
    let mut state = codec::blob(1, crate::restate_services::KEY_STATE.as_bytes())?;
    state.extend(codec::blob(2, &serde_json::to_vec(&fixture.state)?)?);
    start.extend(codec::blob(4, &state)?);
    start.extend(codec::blob(6, fixture.key().as_bytes())?);
    let body = request_wire::encode(&Frame::new(0, start), journal)?;
    let response = client
        .post(url)
        .header("content-type", "application/vnd.restate.invocation.v6")
        .body(body)
        .send()
        .await?;
    check!(eq; response.status(), reqwest::StatusCode::OK);
    codec::frames(&response.bytes().await?)
}

pub(super) fn output(frames: &[Frame]) -> TestResult<Vec<u8>> {
    let frame = frames
        .iter()
        .find(|frame| frame.kind == 0x0401)
        .ok_or("Results output absent")?;
    Ok(codec::field(frame.bytes(14)?, 1)?
        .ok_or("Results output value absent")?
        .to_vec())
}

pub(super) fn require_replay_only(frames: &[Frame]) -> TestResult {
    check!(eq; frames.iter().filter(|frame| frame.kind == 0x0401).count(), 1);
    check!(eq; frames.iter().filter(|frame| frame.kind == 0x0003).count(), 1);
    check!(eq; frames.iter().filter(|frame| frame.kind == 0x0411).count(), 0);
    check!(eq; frames.iter().filter(|frame| frame.kind == 0x0005).count(), 0);
    check!(eq; frames.iter().filter(|frame| frame.kind == 0x0407).count(), 0);
    Ok(())
}

pub(super) fn completion(
    journal: &mut Vec<Frame>,
    frames: &[Frame],
) -> TestResult<ResultsStageOutcome> {
    let run = frames
        .iter()
        .find(|frame| frame.kind == 0x0411)
        .ok_or("Results Run absent")?;
    let state = frames
        .iter()
        .find(|frame| frame.kind == 0x0407)
        .ok_or("retained Results state read absent")?;
    let proposal = frames
        .iter()
        .find(|frame| frame.kind == 0x0005)
        .ok_or("Results Run proposal absent")?;
    check!(eq; frames.iter().filter(|frame| frame.kind == 0x0411).count(), 1);
    check!(eq; frames.iter().filter(|frame| frame.kind == 0x0005).count(), 1);
    check!(codec::field(&run.payload, 12)?.is_none_or(|name| name.is_empty()));
    check!(eq; proposal.number(1)?, run.number(11)?);
    let outcome = serde_json::from_slice(proposal.bytes(14)?)?;
    journal.push(state.clone());
    journal.push(run.clone());
    journal.push(codec::notification(
        0x8011,
        proposal.number(1)?,
        proposal.bytes(14)?,
    )?);
    Ok(outcome)
}

pub(super) fn require_recorded_failure(frames: &[Frame]) -> TestResult {
    let output = frames
        .iter()
        .find(|frame| frame.kind == 0x0401)
        .ok_or("Results failure absent")?;
    let failure = Frame::new(0, output.bytes(15)?.to_vec());
    check!(eq; failure.number(1)?, 409);
    check!(eq; failure.text(2)?, "recorded capture owner conflict");
    check!(eq; frames.iter().filter(|frame| frame.kind == 0x0403).count(), 0);
    require_replay_only(frames)
}

pub(super) fn require_saved_outcome(
    frames: &[Frame],
    fixture: &Fixture,
    outcome: &ResultsStageOutcome,
) -> TestResult {
    check!(eq; frames.iter().filter(|frame| frame.kind == 0x0403).count(), 1);
    let saved = frames
        .iter()
        .find(|frame| frame.kind == 0x0403)
        .ok_or("Results state save absent")?;
    check!(eq; saved.text(1)?, crate::restate_services::KEY_STATE);
    let value = codec::field(saved.bytes(3)?, 1)?.ok_or("Results state value absent")?;
    let mut expected = fixture.state.clone();
    expected.history.results.insert(2026, outcome.clone());
    expected.updated_at = Some("2026-10-14".to_owned());
    check!(eq; serde_json::from_slice::<serde_json::Value>(value)?, serde_json::to_value(expected)?);
    Ok(())
}
