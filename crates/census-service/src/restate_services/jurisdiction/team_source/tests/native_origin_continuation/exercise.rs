use super::transport::{assert_output, call};
use super::{fixture::Fixture, key, ledger, parent::Parent, receipts, NativeNode, TestResult};
use crate::restate_services::wire::{
    OriginHeldContinuationRequest, TeamsAttemptProgress, TeamsSourceOutcome,
};
use census_domain::model::serialized_digest;

pub(super) async fn run(
    fixture: &Fixture,
    client: &reqwest::Client,
    node: &mut NativeNode,
) -> TestResult {
    fixture.seed_national(client, node).await?;
    let held = fixture.hold_origin()?;
    let operation = key(&fixture.request);
    let original = call(
        client,
        &node.ingress,
        "TeamsSource",
        &operation,
        "run",
        "original",
        &fixture.request,
    )
    .await?;
    let outcome: TeamsSourceOutcome = serde_json::from_slice(&original.body)?;
    check!(
        matches!(&outcome, TeamsSourceOutcome::Terminal { message, progress }
        if message.starts_with("origin https://oh.milesplit.com is held by another census-service process")
        && matches!(progress.as_slice(), [TeamsAttemptProgress::Terminal { attempt: 1, outcome: None, .. }]))
    );
    check!(eq; fixture.initial.stats().await.physical_requests(), 0);
    let original_ledger = receipts::phase(&fixture.store, ledger::PHASE)?;
    let parent = Parent::seed(fixture, client, node, &outcome).await?;
    parent.assert_unsettled_refused(client, node).await?;
    drop(held);
    fixture.install_retained_index().await?;
    let continuation = OriginHeldContinuationRequest {
        original: fixture.request.clone(),
        original_outcome_digest: serialized_digest(&outcome)?,
    };
    let continued = call(
        client,
        &node.ingress,
        "TeamsSource",
        &operation,
        "continue_origin_held",
        "continued",
        &continuation,
    )
    .await?;
    let settled: TeamsSourceOutcome = serde_json::from_slice(&continued.body)?;
    check!(
        matches!(&settled, TeamsSourceOutcome::Completed { outcome, .. }
        if outcome.records == 1 && outcome.disposition.is_complete() && outcome.errors.is_empty())
    );
    check!(eq; fixture.continued.stats().await.physical_requests(), 0);
    check!(eq; fixture.continued.stats().await.cache_hits, 1);
    check!(eq; receipts::phase(&fixture.store, ledger::PHASE)?, original_ledger);
    receipts::assert_budget(&fixture.store, &operation)?;
    let reconciled = parent.reconcile(client, node).await?;
    let continued_ledger = receipts::phase(&fixture.store, ledger::origin_continuation::PHASE)?;
    let archive = receipts::archives(fixture.continued.cache_dir())?;
    check!(!archive.is_empty());
    fixture.store.flush()?;
    node.restart(client).await?;
    assert_output(client, node, &original).await?;
    assert_output(client, node, &continued).await?;
    assert_output(client, node, &reconciled).await?;
    parent.assert_replayed(client, node, &reconciled).await?;
    let replay_original = call(
        client,
        &node.ingress,
        "TeamsSource",
        &operation,
        "run",
        "original",
        &fixture.request,
    )
    .await?;
    check!(eq; replay_original.invocation, original.invocation);
    check!(eq; replay_original.body, original.body);
    let replay_continued = call(
        client,
        &node.ingress,
        "TeamsSource",
        &operation,
        "continue_origin_held",
        "continued",
        &continuation,
    )
    .await?;
    check!(eq; replay_continued.invocation, continued.invocation);
    check!(eq; replay_continued.body, continued.body);
    let reattached = call(
        client,
        &node.ingress,
        "TeamsSource",
        &operation,
        "continue_origin_held",
        "reattached",
        &continuation,
    )
    .await?;
    check!(eq; reattached.body, continued.body);
    let original_reattached = call(
        client,
        &node.ingress,
        "TeamsSource",
        &operation,
        "run",
        "original-reattached",
        &fixture.request,
    )
    .await?;
    check!(eq; original_reattached.body, original.body);
    check!(eq; receipts::phase(&fixture.store, ledger::PHASE)?, original_ledger);
    check!(eq; receipts::phase(&fixture.store, ledger::origin_continuation::PHASE)?, continued_ledger);
    check!(eq; receipts::archives(fixture.continued.cache_dir())?, archive);
    receipts::assert_budget(&fixture.store, &operation)?;
    check!(eq; fixture.continued.stats().await.physical_requests(), 0);
    check!(eq; fixture.continued.stats().await.cache_hits, 1);
    println!("native OriginHeld continuation: original={} continuation={} reconciliation={} source={} attempts=2/3 records=1 parent_records=1 original_receipt_unchanged=true network_requests=0 restart_outputs_exact=true", original.invocation, continued.invocation, reconciled.invocation, operation);
    Ok(())
}
