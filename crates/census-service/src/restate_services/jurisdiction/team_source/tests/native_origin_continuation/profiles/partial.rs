use super::{parent, source::Expected, transport, Fixture, NativeNode, TestResult};
use crate::restate_services::wire::JurisdictionState;
use serde_json::Value;

pub(super) async fn run(
    fixture: &Fixture,
    client: &reqwest::Client,
    node: &NativeNode,
    expected: &Expected,
    original: &JurisdictionState,
    initial_progress: &[u8],
) -> TestResult {
    let key = original.identity.as_str();
    let mut pending = original.clone();
    let roster = pending.rosters.as_mut().ok_or("original roster stage")?;
    roster.rosters_committed = 0;
    roster.rosters_remaining = 1;
    roster.errors = vec!["controlled retained roster awaits original stage acknowledgement".into()];
    parent::replace(client, node, key, &pending, Some(initial_progress)).await?;
    let baseline = (
        fixture.store.snapshot().sequence(),
        fixture.store.receipt_count()?,
    );
    let request = serde_json::json!({"original":fixture.request.jurisdiction,"max_steps":128});
    let waiting = call(client, node, key, "profile-partial-stage", &request).await?;
    let value: Value = serde_json::from_slice(&waiting.body)?;
    let count = u64::try_from(expected.members.len())?;
    check!(eq; value["captured"].as_u64(), count.checked_mul(2));
    check!(eq; value["scan_rounds"].as_u64(), Some(2));
    check!(eq; value["source_gaps"].as_u64(), expected.source_gaps.checked_mul(2));
    check!(eq; value["awaiting_rosters"].as_bool(), Some(true));
    check!(eq; value["exhausted"].as_bool(), Some(false));
    let same = call(client, node, key, "profile-partial-unchanged", &request).await?;
    check!(eq; same.body, waiting.body);
    parent::replace(client, node, key, original, Some(&same.body)).await?;
    let finished = call(client, node, key, "profile-stage-acknowledged", &request).await?;
    let value: Value = serde_json::from_slice(&finished.body)?;
    check!(eq; value["captured"].as_u64(), count.checked_mul(3));
    check!(eq; value["scan_rounds"].as_u64(), Some(3));
    check!(eq; value["source_gaps"].as_u64(), expected.source_gaps.checked_mul(3));
    check!(eq; value["awaiting_rosters"].as_bool(), Some(false));
    check!(eq; value["exhausted"].as_bool(), Some(true));
    check!(eq; value["failed"].as_u64(), Some(0));
    check!(eq; (fixture.store.snapshot().sequence(), fixture.store.receipt_count()?), baseline);
    check!(eq; fixture.continued.stats().await.physical_requests(), 0);
    Ok(())
}

async fn call(
    client: &reqwest::Client,
    node: &NativeNode,
    key: &str,
    idempotency: &str,
    request: &Value,
) -> TestResult<transport::Reply> {
    transport::call(
        client,
        &node.ingress,
        "JurisdictionCensus",
        key,
        "public_profiles",
        idempotency,
        request,
    )
    .await
}
