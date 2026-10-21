use super::*;

pub(super) fn initial(
    state: &JurisdictionState,
    request: &JurisdictionRequest,
) -> TestResult<Vec<Frame>> {
    let mut eager = codec::blob(1, b"state")?;
    eager.extend(codec::blob(
        14,
        &codec::value(&serde_json::to_vec(state)?)?,
    )?);
    Ok(vec![
        Frame::new(
            0x0400,
            codec::blob(14, &codec::value(&serde_json::to_vec(request)?)?)?,
        ),
        Frame::new(0x0407, eager),
    ])
}

pub(super) async fn drive(
    client: &reqwest::Client,
    url: &str,
    fixture: &fixture::Fixture,
    journal: &mut Vec<Frame>,
    mode: &str,
) -> TestResult<(Frame, Vec<Frame>, Vec<String>, Vec<Frame>)> {
    let mut saved = Vec::new();
    let mut calls = Vec::new();
    let mut before_save = Vec::new();
    for _ in 0..12 {
        for frame in
            super::super::replay::invoke(client, url, &fixture.state.identity, journal).await?
        {
            match frame.kind {
                0x0411 => journal.push(frame),
                0x0005 => journal.push(run_completion(&frame)?),
                0x040D => {
                    let response = fixture.response(&frame, mode)?;
                    calls.push(format!(
                        "{}:{}:{}",
                        frame.text(1)?,
                        frame.text(2)?,
                        frame.text(5)?
                    ));
                    let mut invocation = codec::integer(1, frame.number(10)?);
                    invocation.extend(codec::blob(16, b"immutable-source-fixture")?);
                    journal.push(frame.clone());
                    journal.push(Frame::new(0x800E, invocation));
                    journal.push(codec::notification(0x800D, frame.number(11)?, &response)?);
                }
                0x0403 => {
                    check!(eq; frame.text(1)?, "state");
                    before_save = journal.clone();
                    saved.push(frame.clone());
                    journal.push(frame);
                }
                0x0401 => return Ok((frame, saved, calls, before_save)),
                0x0002 => return Err(format!("SDK suffix failed: {}", frame.text(2)?).into()),
                0x0001 | 0x0003 => {}
                _ => return Err(format!("unexpected suffix frame {frame:?}").into()),
            }
        }
    }
    Err("suffix did not settle within journal bound".into())
}

fn run_completion(frame: &Frame) -> TestResult<Frame> {
    if let Some(value) = codec::field(&frame.payload, 14)? {
        return codec::notification(0x8011, frame.number(1)?, value);
    }
    let mut payload = codec::integer(1, frame.number(1)?);
    payload.extend(codec::blob(6, frame.bytes(15)?)?);
    Ok(Frame::new(0x8011, payload))
}

pub(super) fn output(frame: &Frame) -> TestResult<JurisdictionState> {
    let content = codec::field(frame.bytes(14)?, 1)?.ok_or("output missing content")?;
    Ok(serde_json::from_slice(content)?)
}

pub(super) fn saved(frames: &[Frame]) -> TestResult<JurisdictionState> {
    let [frame] = frames else {
        return Err("expected one saved parent state".into());
    };
    let content = codec::field(frame.bytes(3)?, 1)?.ok_or("state missing content")?;
    Ok(serde_json::from_slice(content)?)
}
