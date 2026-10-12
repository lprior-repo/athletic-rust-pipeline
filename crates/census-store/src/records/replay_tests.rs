use super::*;

#[test]
fn completed_unit_replays_before_build_finishes() -> TestResult {
    let (_dir, store) = store()?;
    completed(&store)?;
    check!(eq; store.get_build_record(&id('b'))?.ok_or("missing")?.state(), crate::BuildState::Building);
    check!(store.unit_replay_complete(&id('c'), &[])?);
    Ok(())
}
