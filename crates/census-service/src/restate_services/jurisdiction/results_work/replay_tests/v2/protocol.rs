use super::*;
use crate::restate_services::national::publication_tests::codec::{self, Frame};

pub(in crate::restate_services::jurisdiction::results_work::replay_tests) fn acknowledge(
    journal: &mut Vec<Frame>,
    frames: &[Frame],
    name: &str,
) -> TestResult<Vec<u8>> {
    let run = require_run(frames, name)?;
    for state in frames.iter().filter(|frame| frame.kind == 0x0407) {
        journal.push(state.clone());
    }
    let proposal = proposal(frames)?;
    check!(eq; proposal.number(1)?, run.number(11)?);
    let value = proposal.bytes(14)?.to_vec();
    journal.push(run.clone());
    journal.push(codec::notification(0x8011, proposal.number(1)?, &value)?);
    Ok(value)
}

pub(super) fn leave_unacknowledged(
    journal: &mut Vec<Frame>,
    frames: &[Frame],
    name: &str,
) -> TestResult<ResultsStageOutcome> {
    let run = require_run(frames, name)?;
    let proposal = proposal(frames)?;
    check!(eq; proposal.number(1)?, run.number(11)?);
    journal.push(run.clone());
    Ok(serde_json::from_slice(proposal.bytes(14)?)?)
}

pub(super) fn acknowledge_reexecution(
    journal: &mut Vec<Frame>,
    frames: &[Frame],
) -> TestResult<ResultsStageOutcome> {
    check!(eq; frames.iter().filter(|frame| frame.kind == 0x0411).count(), 0);
    let proposal = proposal(frames)?;
    let run = journal
        .iter()
        .rev()
        .find(|frame| frame.kind == 0x0411)
        .ok_or("pending run absent")?;
    check!(eq; run.text(12)?, "results-v2-execute");
    check!(eq; proposal.number(1)?, run.number(11)?);
    let result = serde_json::from_slice(proposal.bytes(14)?)?;
    journal.push(codec::notification(
        0x8011,
        proposal.number(1)?,
        proposal.bytes(14)?,
    )?);
    Ok(result)
}

pub(super) fn overwrite_plan_notification(journal: &mut [Frame], bytes: &[u8]) -> TestResult {
    let run = journal
        .iter()
        .find(|frame| frame.kind == 0x0411)
        .ok_or("plan run absent")?;
    check!(eq; run.text(12)?, "results-v2-plan");
    let id = run.number(11)?;
    let notification = journal
        .iter_mut()
        .find(|frame| frame.kind == 0x8011)
        .ok_or("plan notification absent")?;
    *notification = codec::notification(0x8011, id, bytes)?;
    Ok(())
}

pub(super) fn require_run<'a>(frames: &'a [Frame], name: &str) -> TestResult<&'a Frame> {
    let run = frames
        .iter()
        .find(|frame| frame.kind == 0x0411)
        .ok_or("run command absent")?;
    check!(eq; run.text(12)?, name);
    check!(eq; frames.iter().filter(|frame| frame.kind == 0x0411).count(), 1);
    check!(eq; frames.iter().filter(|frame| frame.kind == 0x0005).count(), 1);
    check!(eq; frames.iter().filter(|frame| frame.kind == 0x0403).count(), 0);
    check!(eq; frames.iter().filter(|frame| frame.kind == 0x0401).count(), 0);
    Ok(run)
}

pub(super) fn proposal(frames: &[Frame]) -> TestResult<&Frame> {
    let proposal = frames
        .iter()
        .find(|frame| frame.kind == 0x0005)
        .ok_or("run proposal absent")?;
    check!(eq; frames.iter().filter(|frame| frame.kind == 0x0005).count(), 1);
    Ok(proposal)
}
