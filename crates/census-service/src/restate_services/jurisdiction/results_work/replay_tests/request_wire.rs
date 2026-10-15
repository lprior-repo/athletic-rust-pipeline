use super::TestResult;
use crate::restate_services::national::publication_tests::codec::{self, Frame};
use crate::restate_services::results_arms::plan_v2::MAX_PLAN_BYTES;

pub(super) fn encode(start: &Frame, journal: &[Frame]) -> TestResult<Vec<u8>> {
    let length = admitted_length(start, journal)?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(length)?;
    for frame in std::iter::once(start).chain(journal) {
        bytes.extend(frame.kind.to_be_bytes());
        bytes.extend(frame.flags.to_be_bytes());
        bytes.extend(u32::try_from(frame.payload.len())?.to_be_bytes());
        bytes.extend_from_slice(&frame.payload);
    }
    Ok(bytes)
}

fn admitted_length(start: &Frame, journal: &[Frame]) -> TestResult<usize> {
    let count = journal
        .len()
        .checked_add(1)
        .ok_or("fixture frame count overflow")?;
    check!(count <= codec::MAX_FRAMES, "fixture frame budget exhausted");
    let mut length = 0_usize;
    let mut largest_completion = 0_usize;
    for frame in std::iter::once(start).chain(journal) {
        let completion = completion_length(frame)?;
        largest_completion = largest_completion.max(completion);
        length = length
            .checked_add(frame.wire_size()?)
            .ok_or("fixture request length overflow")?;
    }
    let overhead = length
        .checked_sub(largest_completion)
        .ok_or("fixture framing length underflow")?;
    check!(
        overhead <= codec::MAX_WIRE_BYTES,
        "fixture framing byte budget exhausted"
    );
    Ok(length)
}

fn completion_length(frame: &Frame) -> TestResult<usize> {
    if frame.kind != 0x8011 {
        return Ok(0);
    }
    let Some(value) = codec::field(&frame.payload, 5)? else {
        return Ok(0);
    };
    let content = codec::field(value, 1)?.ok_or("fixture completion value absent")?;
    let limit = MAX_PLAN_BYTES
        .checked_add(1)
        .ok_or("fixture completion limit overflow")?;
    check!(
        content.len() <= limit,
        "fixture completion byte budget exhausted"
    );
    Ok(content.len())
}
