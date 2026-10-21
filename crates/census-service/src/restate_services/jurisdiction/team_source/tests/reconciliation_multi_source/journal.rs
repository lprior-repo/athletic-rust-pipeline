use super::*;

pub(super) async fn drive(
    client: &reqwest::Client,
    url: &str,
    fixture: &Fixture,
    journal: &mut Vec<Frame>,
) -> TestResult<(Frame, Vec<Frame>, Vec<String>)> {
    let mut saved = Vec::new();
    let mut calls = Vec::new();
    for _ in 0..10 {
        for frame in
            super::super::replay::invoke(client, url, &fixture.state.identity, journal).await?
        {
            match frame.kind {
                0x0411 => journal.push(frame),
                0x0005 => journal.push(codec::notification(
                    0x8011,
                    frame.number(1)?,
                    frame.bytes(14)?,
                )?),
                0x040D => {
                    let response = fixture.response(&frame)?;
                    calls.push(
                        frame
                            .text(5)?
                            .rsplit('/')
                            .next()
                            .ok_or("source key absent")?
                            .into(),
                    );
                    let mut invocation = codec::integer(1, frame.number(10)?);
                    invocation.extend(codec::blob(16, b"retained-source-fixture")?);
                    journal.push(frame.clone());
                    journal.push(Frame::new(0x800E, invocation));
                    journal.push(codec::notification(0x800D, frame.number(11)?, &response)?);
                }
                0x0403 => {
                    check!(eq; frame.text(1)?, "state");
                    saved.push(frame.clone());
                    journal.push(frame);
                }
                0x0401 => return Ok((frame, saved, calls)),
                0x0002 => {
                    return Err(format!("SDK reconciliation failed: {}", frame.text(2)?).into())
                }
                0x0001 | 0x0003 => {}
                _ => return Err(format!("unexpected reconciliation frame {frame:?}").into()),
            }
        }
    }
    Err("reconciliation did not settle within journal bound".into())
}
