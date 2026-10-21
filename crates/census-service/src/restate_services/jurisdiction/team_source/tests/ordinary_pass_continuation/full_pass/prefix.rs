use super::*;
use crate::restate_services::jurisdiction::team_collection;
use crate::restate_services::wire::{TeamsSourceOutcome, TeamsSourceRequest};
use census_crawl::CollectionDisposition;

pub(super) fn completed(fixture: &Fixture) -> TestResult<Vec<Frame>> {
    let mut frames = journal::initial(&fixture.state, &fixture.source.jurisdiction)?;
    frames.push(Frame::new(0x0411, codec::integer(11, 2)));
    frames.push(codec::notification(
        0x8011,
        2,
        &serde_json::to_vec(&fixture.source.observed_on)?,
    )?);
    frames.push(saved(&fixture.state)?);
    let mut aggregate = team_collection::empty_outcome(
        &fixture.source.observed_on,
        CollectionDisposition::Complete,
    );
    let mut failures = Vec::new();
    for (source, invocation, completion) in [("ihsa", 3, 4), ("milesplit", 5, 6)] {
        let mut input = fixture.source.clone();
        input.source = source.into();
        let outcome = ledger::status(&fixture.store, &key(&input), Some(&input))?
            .ok_or("immutable source output absent")?;
        record_source(&mut frames, &input, invocation, completion, &outcome)?;
        team_collection::retain(source, outcome, &mut aggregate, &mut failures)?;
    }
    let mut state = fixture.state.clone();
    state.teams = team_collection::finish(aggregate, failures, &fixture.source.observed_on);
    check!(eq; serde_json::to_vec(&state)?, serde_json::to_vec(&fixture.state)?);
    frames.push(saved(&state)?);
    check!(eq; frames.len(), 12);
    Ok(frames)
}

pub(super) fn tamper(frames: &mut [Frame], fixture: &Fixture) -> TestResult {
    let first = frames
        .iter_mut()
        .find(|frame| frame.kind == 0x040D)
        .ok_or("old Call absent")?;
    let mut input = fixture.source.clone();
    input.source = "ihsa".into();
    *first = source_call(&input, "contacts", 3, 4)?;
    Ok(())
}

fn record_source(
    frames: &mut Vec<Frame>,
    input: &TeamsSourceRequest,
    invocation: u64,
    completion: u64,
    outcome: &TeamsSourceOutcome,
) -> TestResult {
    frames.push(source_call(input, "run", invocation, completion)?);
    let mut id = codec::integer(1, invocation);
    id.extend(codec::blob(16, b"controlled-original-source")?);
    frames.push(Frame::new(0x800E, id));
    frames.push(codec::notification(
        0x800D,
        completion,
        &serde_json::to_vec(outcome)?,
    )?);
    Ok(())
}

fn source_call(
    input: &TeamsSourceRequest,
    handler: &str,
    invocation: u64,
    completion: u64,
) -> TestResult<Frame> {
    let mut payload = codec::blob(1, b"TeamsSource")?;
    payload.extend(codec::blob(2, handler.as_bytes())?);
    payload.extend(codec::blob(3, &serde_json::to_vec(input)?)?);
    payload.extend(codec::blob(5, key(input).as_bytes())?);
    payload.extend(codec::integer(10, invocation));
    payload.extend(codec::integer(11, completion));
    Ok(Frame::new(0x040D, payload))
}

fn saved(state: &JurisdictionState) -> TestResult<Frame> {
    let mut payload = codec::blob(1, b"state")?;
    payload.extend(codec::blob(3, &codec::value(&serde_json::to_vec(state)?)?)?);
    Ok(Frame::new(0x0403, payload))
}
