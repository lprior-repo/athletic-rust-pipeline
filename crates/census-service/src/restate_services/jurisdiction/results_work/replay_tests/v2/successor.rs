use super::branch_support as support;
use super::*;
use crate::spawn::TaskReport;
use census_domain::model::AcquisitionFailureV1;
use std::sync::Arc;

#[tokio::test(flavor = "current_thread")]
async fn later_distinct_sdk_invocation_after_actual_drain_alone_creates_ordinal_two() -> TestResult
{
    let mut fixture = Fixture::v2()?;
    let prepared = fixture::seed_prepared(&fixture)?;
    support::create_missing_archive_parents(&fixture, &prepared)?;
    let resources = fixture.region.coordinator().remaining();
    let original_store = Arc::clone(&fixture.store);
    let old_region = Arc::clone(&fixture.region);
    let mut saved = None;
    let saved_out = &mut saved;
    let f = &fixture;
    let prepared_ref = &prepared;
    let first = endpoint::serve(&fixture, |client, url| async move {
        let mut journal = endpoint::input(f)?;
        let frames = endpoint::invoke(&client, &url, f, &journal).await?;
        let plan_bytes = protocol::acknowledge(&mut journal, &frames, "results-v2-plan")?;
        support::plan(&plan_bytes, prepared_ref, 1)?;
        super::super::readmit_same_key(f).await?;
        let frames = endpoint::invoke(&client, &url, f, &journal).await?;
        let bytes = protocol::acknowledge(&mut journal, &frames, "results-v2-execute")?;
        let outcome: ResultsStageOutcome = serde_json::from_slice(&bytes)?;
        let truth = support::truth(f, prepared_ref, 1)?;
        support::missing(&truth, &plan_bytes)?;
        support::note(&outcome, "retention_failed", &truth.receipt)?;
        support::no_successor(f, prepared_ref, 1)?;
        super::super::readmit_same_key(f).await?;
        let frames = endpoint::invoke(&client, &url, f, &journal).await?;
        endpoint::require_replay_only(&frames)?;
        endpoint::require_saved_outcome(&frames, f, &outcome)?;
        check!(eq; endpoint::output(&frames)?, bytes);
        *saved_out = Some(support::saved_state(&frames)?);
        Ok(())
    })
    .await;
    let report = old_region.drain(WAIT).await?;
    first?;
    check!(eq; report, TaskReport { accepted: 2, completed: 2, cancelled: 0, aborted: 0, panicked: 0, failed: 0, still_running: 0, timed_out: 0, remaining: 0 });
    check!(eq; report.accepted, report.disjoint_total());
    check!(eq; old_region.coordinator().remaining(), resources);
    let original = support::truth(&fixture, &prepared, 1)?;
    fixture.install_continuation(saved.ok_or("saved continuation state absent")?)?;
    check!(Arc::ptr_eq(&original_store, &fixture.store));
    check!(!Arc::ptr_eq(&old_region, &fixture.region));
    let resources = fixture.region.coordinator().remaining();
    let f = &fixture;
    let later = endpoint::serve(&fixture, |client, url| async move {
        let mut journal = endpoint::input(f)?;
        let before_plan = acquisition(f)?;
        let frames = support::invoke_later(&client, &url, f, &journal).await?;
        support::plan(&protocol::acknowledge(&mut journal, &frames, "results-v2-plan")?, &prepared, 0)?;
        check!(eq; acquisition(f)?, before_plan);
        super::super::readmit_same_key(f).await?;
        let frames = support::invoke_later(&client, &url, f, &journal).await?;
        let bytes = protocol::acknowledge(&mut journal, &frames, "results-v2-execute")?;
        let outcome: ResultsStageOutcome = serde_json::from_slice(&bytes)?;
        let second = support::truth(f, &prepared, 2)?;
        check!(eq; second.terminal.terminal(), &AcquisitionTerminal::NotDispatched { failure: AcquisitionFailureV1::AdmissionRefused });
        support::unchanged(f, &prepared, 1, &original)?;
        support::no_successor(f, &prepared, 2)?;
        check!(eq; f.fetcher.stats().await.physical_requests(), 0);
        super::super::readmit_same_key(f).await?;
        let before_replay = snapshot(f).await?;
        let frames = support::invoke_later(&client, &url, f, &journal).await?;
        endpoint::require_replay_only(&frames)?;
        endpoint::require_saved_outcome(&frames, f, &outcome)?;
        check!(eq; endpoint::output(&frames)?, bytes);
        support::unchanged(f, &prepared, 1, &original)?;
        support::unchanged(f, &prepared, 2, &second)?;
        support::no_successor(f, &prepared, 2)?;
        check!(eq; snapshot(f).await?, before_replay);
        super::super::readmit_same_key(f).await?;
        Ok(())
    }).await;
    super::super::finish(&fixture, later, 2, resources).await
}
