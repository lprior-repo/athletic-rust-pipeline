use super::branch_support as support;
use super::*;

#[tokio::test(flavor = "current_thread")]
async fn retained_v2_fixed_plan_lost_ack_reexecutes_exact_retained_truth_and_bytes() -> TestResult {
    let fixture = Fixture::v2()?;
    let prepared = fixture::seed_prepared(&fixture)?;
    support::publish_exact(&fixture, &prepared)?;
    let resources = fixture.region.coordinator().remaining();
    let f = &fixture;
    let result = endpoint::serve(&fixture, |client, url| async move {
        let mut journal = endpoint::input(f)?;
        let before_plan = acquisition(f)?;
        let frames = endpoint::invoke(&client, &url, f, &journal).await?;
        let plan_bytes = protocol::acknowledge(&mut journal, &frames, "results-v2-plan")?;
        support::plan(&plan_bytes, &prepared, 1)?;
        check!(eq; acquisition(f)?, before_plan);
        super::super::readmit_same_key(f).await?;
        let frames = endpoint::invoke(&client, &url, f, &journal).await?;
        let original = protocol::leave_unacknowledged(&mut journal, &frames, "results-v2-execute")?;
        let original_bytes = protocol::proposal(&frames)?.bytes(14)?.to_vec();
        let truth = support::truth(f, &prepared, 1)?;
        support::retained(&truth, &plan_bytes)?;
        support::note(&original, "retained", &truth.receipt)?;
        support::no_successor(f, &prepared, 1)?;
        super::super::readmit_same_key(f).await?;
        let before_replay = snapshot(f).await?;
        let frames = endpoint::invoke(&client, &url, f, &journal).await?;
        check!(eq; protocol::proposal(&frames)?.bytes(14)?, original_bytes.as_slice());
        let replay = protocol::acknowledge_reexecution(&mut journal, &frames)?;
        check!(eq; replay, original);
        support::unchanged(f, &prepared, 1, &truth)?;
        support::no_successor(f, &prepared, 1)?;
        check!(eq; snapshot(f).await?, before_replay);
        check!(eq; f.fetcher.stats().await.physical_requests(), 0);
        super::super::readmit_same_key(f).await?;
        let frames = endpoint::invoke(&client, &url, f, &journal).await?;
        endpoint::require_replay_only(&frames)?;
        endpoint::require_saved_outcome(&frames, f, &original)?;
        check!(eq; endpoint::output(&frames)?, original_bytes);
        check!(eq; snapshot(f).await?, before_replay);
        super::super::readmit_same_key(f).await?;
        Ok(())
    })
    .await;
    super::super::finish(&fixture, result, 3, resources).await
}

#[tokio::test(flavor = "current_thread")]
async fn retained_v2_unavailable_reexecution_preserves_retained_fact_and_replays_unavailable_bytes(
) -> TestResult {
    let fixture = Fixture::v2()?;
    let prepared = fixture::seed_prepared(&fixture)?;
    let body = support::publish_exact(&fixture, &prepared)?;
    let resources = fixture.region.coordinator().remaining();
    let f = &fixture;
    let result = endpoint::serve(&fixture, |client, url| async move {
        let mut journal = endpoint::input(f)?;
        let frames = endpoint::invoke(&client, &url, f, &journal).await?;
        let plan_bytes = protocol::acknowledge(&mut journal, &frames, "results-v2-plan")?;
        support::plan(&plan_bytes, &prepared, 1)?;
        super::super::readmit_same_key(f).await?;
        let frames = endpoint::invoke(&client, &url, f, &journal).await?;
        let retained = protocol::leave_unacknowledged(&mut journal, &frames, "results-v2-execute")?;
        let retained_bytes = protocol::proposal(&frames)?.bytes(14)?.to_vec();
        let truth = support::truth(f, &prepared, 1)?;
        support::retained(&truth, &plan_bytes)?;
        support::note(&retained, "retained", &truth.receipt)?;
        super::super::readmit_same_key(f).await?;
        std::fs::remove_file(&body)?;
        let before_unavailable = snapshot(f).await?;
        let frames = endpoint::invoke(&client, &url, f, &journal).await?;
        check!(eq; frames.iter().filter(|frame| frame.kind == 0x0411).count(), 0);
        let unavailable_bytes = protocol::proposal(&frames)?.bytes(14)?.to_vec();
        check!(unavailable_bytes != retained_bytes);
        let unavailable: ResultsStageOutcome = serde_json::from_slice(&unavailable_bytes)?;
        support::unavailable(&unavailable, &truth.receipt)?;
        support::unchanged(f, &prepared, 1, &truth)?;
        support::no_successor(f, &prepared, 1)?;
        check!(eq; snapshot(f).await?, before_unavailable);
        check!(!body.exists());
        super::super::readmit_same_key(f).await?;
        let frames = endpoint::invoke(&client, &url, f, &journal).await?;
        check!(eq; protocol::proposal(&frames)?.bytes(14)?, unavailable_bytes.as_slice());
        check!(eq; protocol::acknowledge_reexecution(&mut journal, &frames)?, unavailable);
        support::unchanged(f, &prepared, 1, &truth)?;
        support::no_successor(f, &prepared, 1)?;
        check!(eq; snapshot(f).await?, before_unavailable);
        check!(eq; f.fetcher.stats().await.physical_requests(), 0);
        super::super::readmit_same_key(f).await?;
        let frames = endpoint::invoke(&client, &url, f, &journal).await?;
        endpoint::require_replay_only(&frames)?;
        endpoint::require_saved_outcome(&frames, f, &unavailable)?;
        check!(eq; endpoint::output(&frames)?, unavailable_bytes);
        check!(eq; snapshot(f).await?, before_unavailable);
        check!(!body.exists());
        super::super::readmit_same_key(f).await?;
        Ok(())
    })
    .await;
    super::super::finish(&fixture, result, 4, resources).await
}
