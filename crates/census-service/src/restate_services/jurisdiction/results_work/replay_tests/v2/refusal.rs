use super::*;
use crate::restate_services::results_arms::plan_v2::MAX_PLAN_BYTES;

#[tokio::test(flavor = "current_thread")]
async fn oversized_completed_j0_bytes_refuse_at_real_sdk_decode_before_j1() -> TestResult {
    let fixture = Fixture::v2()?;
    fixture::seed_prepared(&fixture)?;
    let resources = fixture.region.coordinator().remaining();
    let f = &fixture;
    let result = endpoint::serve(&fixture, |client, url| async move {
        let mut journal = endpoint::input(f)?;
        let frames = endpoint::invoke(&client, &url, f, &journal).await?;
        let mut encoded = protocol::acknowledge(&mut journal, &frames, "results-v2-plan")?;
        encoded.resize(
            MAX_PLAN_BYTES
                .checked_add(1)
                .ok_or("fixture next plan byte overflow")?,
            b' ',
        );
        protocol::overwrite_plan_notification(&mut journal, &encoded)?;
        let before = snapshot(f).await?;
        let frames = endpoint::invoke_frames(&client, &url, f, &journal).await?;
        check!(eq; frames.iter().filter(|frame| frame.kind == 0x0002).count(), 1);
        check!(eq; frames.iter().filter(|frame| frame.kind == 0x0411).count(), 0);
        check!(eq; frames.iter().filter(|frame| frame.kind == 0x0005).count(), 0);
        check!(eq; snapshot(f).await?, before);
        super::super::readmit_same_key(f).await?;
        Ok(())
    })
    .await;
    super::super::finish(&fixture, result, 1, resources).await
}

#[tokio::test(flavor = "current_thread")]
async fn j0_encoding_admission_refusal_never_dispatches_execute_or_changes_acquisitions(
) -> TestResult {
    let fixture = Fixture::v2()?;
    fixture::seed_prepared(&fixture)?;
    fixture::pending_overflow(&fixture)?;
    let resources = fixture.region.coordinator().remaining();
    let before = acquisition(&fixture)?;
    let f = &fixture;
    let result = endpoint::serve(&fixture, |client, url| async move {
        let journal = endpoint::input(f)?;
        let frames = endpoint::invoke(&client, &url, f, &journal).await?;
        protocol::require_run(&frames, "results-v2-plan")?;
        let failure = protocol::proposal(&frames)?.bytes(15)?;
        let failure = Frame::new(0, failure.to_vec());
        check!(failure.text(2)?.contains("encoded plan bytes"));
        check!(eq; acquisition(f)?, before);
        super::super::readmit_same_key(f).await?;
        Ok(())
    })
    .await;
    let report = fixture.region.drain(WAIT).await?;
    result?;
    check!(eq; report.accepted, 1);
    check!(eq; report.completed, 1);
    check!(eq; report.failed, 1);
    check!(eq; report.still_running, 0);
    check!(eq; report.remaining, 0);
    check!(eq; report.accepted, report.disjoint_total());
    check!(eq; fixture.region.coordinator().remaining(), resources);
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn exact_cap_completed_j0_bytes_are_decoded_by_the_actual_sdk_before_execute() -> TestResult {
    let fixture = Fixture::v2()?;
    let prepared = fixture::seed_prepared(&fixture)?;
    let resources = fixture.region.coordinator().remaining();
    let f = &fixture;
    let result = endpoint::serve(&fixture, |client, url| async move {
        let mut journal = endpoint::input(f)?;
        let frames = endpoint::invoke(&client, &url, f, &journal).await?;
        let mut encoded = protocol::acknowledge(&mut journal, &frames, "results-v2-plan")?;
        encoded.resize(MAX_PLAN_BYTES, b' ');
        protocol::overwrite_plan_notification(&mut journal, &encoded)?;
        let frames = endpoint::invoke(&client, &url, f, &journal).await?;
        let outcome = protocol::acknowledge(&mut journal, &frames, "results-v2-execute")?;
        let outcome: ResultsStageOutcome = serde_json::from_slice(&outcome)?;
        require_failed_rows(&outcome, 1, &require_closed(f, &prepared)?)?;
        super::super::readmit_same_key(f).await?;
        Ok(())
    })
    .await;
    super::super::finish(&fixture, result, 2, resources).await
}
