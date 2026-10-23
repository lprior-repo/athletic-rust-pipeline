use super::{parent, source::Expected, transport, Fixture, NativeNode, TestResult};
use crate::restate_services::wire::JurisdictionState;
use census_store::Table;
use serde_json::Value;

pub(super) async fn run(
    fixture: &Fixture,
    client: &reqwest::Client,
    node: &mut NativeNode,
    expected: &Expected,
) -> TestResult {
    fixture.seed_national(client, node).await?;
    let (key, original) = parent::seed(fixture, client, node).await?;
    refusals(fixture, client, node, &key).await?;
    let single = request(fixture, 1);
    let first = invoke(client, node, &key, "profile-first", &single).await?;
    let value: Value = serde_json::from_slice(&first.body)?;
    let (_, next_member) = expected
        .members
        .first()
        .ok_or("first actual cohort member")?;
    check!(eq; value["captured"].as_u64(), Some(1));
    check!(eq; value["cursor"]["packet"].as_str(), Some(expected.packet.as_str()));
    check!(eq; value["cursor"]["member"].as_u64(), Some(u64::try_from(*next_member)?));
    check!(eq; value["exhausted"].as_bool(), Some(false));
    let checkpoint = witness(fixture)?;
    let repeated = invoke(client, node, &key, "profile-first", &single).await?;
    check!(eq; repeated.invocation, first.invocation);
    check!(eq; repeated.body, first.body);
    check!(eq; witness(fixture)?, checkpoint);
    let batch = request(fixture, 128);
    let resumed = invoke(client, node, &key, "profile-resume", &batch).await?;
    let completed: Value = serde_json::from_slice(&resumed.body)?;
    complete(&completed, expected)?;
    let checkpoint = witness(fixture)?;
    fixture.store.flush()?;
    node.restart(client).await?;
    transport::assert_output(client, node, &first).await?;
    transport::assert_output(client, node, &resumed).await?;
    let repeated = invoke(client, node, &key, "profile-resume", &batch).await?;
    check!(eq; repeated.invocation, resumed.invocation);
    check!(eq; repeated.body, resumed.body);
    let fresh = invoke(client, node, &key, "profile-finished", &batch).await?;
    check!(eq; fresh.body, resumed.body);
    check!(eq; witness(fixture)?, checkpoint);
    unchanged_parent(client, node, &key, &original).await?;
    check!(eq; fixture.continued.stats().await.physical_requests(), 0);
    check!(eq; fixture.initial.stats().await.physical_requests(), 0);
    check!(eq; fixture.store.walk_table(Table::Performances)?.rows, 0);
    super::partial::run(fixture, client, node, expected, &original, &fresh.body).await?;
    Ok(())
}

fn request(fixture: &Fixture, max_steps: u16) -> Value {
    serde_json::json!({
        "original": fixture.request.jurisdiction,
        "max_steps": max_steps,
    })
}

async fn invoke(
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

async fn refusals(
    fixture: &Fixture,
    client: &reqwest::Client,
    node: &NativeNode,
    key: &str,
) -> TestResult {
    let baseline = witness(fixture)?;
    let mut wrong = fixture.request.jurisdiction.clone();
    wrong.refresh = !wrong.refresh;
    for (idempotency, body, detail) in [
        (
            "profile-zero",
            request(fixture, 0),
            "max_steps must be in 1..=128",
        ),
        (
            "profile-wrong-full-request",
            serde_json::json!({ "original": wrong, "max_steps": 1 }),
            "jurisdiction differs from full bound national request",
        ),
    ] {
        let (status, reply) = transport::call_status(
            client,
            &node.ingress,
            "JurisdictionCensus",
            key,
            "public_profiles",
            idempotency,
            &body,
        )
        .await?;
        check!(status.is_client_error() || status.is_server_error());
        check!(String::from_utf8_lossy(&reply.body).contains(detail));
        check!(eq; witness(fixture)?, baseline);
        check!(eq; fixture.continued.stats().await.physical_requests(), 0);
    }
    Ok(())
}

fn complete(value: &Value, expected: &Expected) -> TestResult {
    check!(eq; value["captured"].as_u64(), Some(u64::try_from(expected.members.len())?));
    check!(eq; value["failed"].as_u64(), Some(0));
    check!(eq; value["source_gaps"].as_u64(), Some(expected.source_gaps));
    check!(eq; value["not_found"].as_u64(), Some(0));
    check!(eq; value["stopped"].as_bool(), Some(false));
    check!(eq; value["exhausted"].as_bool(), Some(true));
    check!(eq; value["cursor"]["packet"].as_str(), Some(expected.packet.as_str()));
    check!(eq; value["cursor"]["finished"].as_bool(), Some(true));
    Ok(())
}

fn witness(fixture: &Fixture) -> TestResult<(u64, u64)> {
    Ok((
        fixture.store.snapshot().sequence(),
        fixture.store.receipt_count()?,
    ))
}

async fn unchanged_parent(
    client: &reqwest::Client,
    node: &NativeNode,
    key: &str,
    expected: &JurisdictionState,
) -> TestResult {
    let reply = transport::call_empty(
        client,
        &node.ingress,
        "JurisdictionCensus",
        key,
        "state",
        "profile-parent-after",
    )
    .await?;
    let actual: JurisdictionState = serde_json::from_slice(&reply.body)?;
    check!(eq; actual.identity, key);
    check!(eq; serde_json::to_value(actual)?, serde_json::to_value(expected)?);
    Ok(())
}
