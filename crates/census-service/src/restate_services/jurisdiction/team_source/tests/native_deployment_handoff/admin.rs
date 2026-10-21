use super::{Evidence, NativeNode, TestResult};
use serde_json::{json, Value};
use std::time::Duration;

fn validate_id(id: &str) -> TestResult {
    check!(
        !id.is_empty()
            && id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    );
    Ok(())
}

pub(super) async fn query(
    client: &reqwest::Client,
    node: &NativeNode,
    sql: &str,
) -> TestResult<Vec<Value>> {
    let response = client
        .post(format!("{}/query", node.admin))
        .header("accept", "application/json")
        .json(&json!({"query": sql}))
        .send()
        .await?;
    let status = response.status();
    let body = response.bytes().await?;
    check!(body.len() <= 1_048_576);
    check!(eq; status, reqwest::StatusCode::OK, "native admin SQL failed: {}", String::from_utf8_lossy(&body));
    let value: Value = serde_json::from_slice(&body)?;
    Ok(value["rows"]
        .as_array()
        .ok_or("native query rows absent")?
        .clone())
}

pub(super) async fn invocation(
    client: &reqwest::Client,
    node: &NativeNode,
    id: &str,
) -> TestResult<Value> {
    validate_id(id)?;
    let sql = format!("SELECT id,target,status,pinned_deployment_id,pinned_service_protocol_version FROM sys_invocation WHERE id='{id}'");
    let rows = query(client, node, &sql).await?;
    let [row] = rows.as_slice() else {
        return Err(format!("native invocation {id} expected one row, got {rows:?}").into());
    };
    Ok(row.clone())
}

pub(super) async fn journal(
    client: &reqwest::Client,
    node: &NativeNode,
    id: &str,
) -> TestResult<Vec<Value>> {
    validate_id(id)?;
    query(client, node, &format!("SELECT index,entry_type,name,raw,version,entry_json FROM sys_journal WHERE id='{id}' ORDER BY index")).await
}

pub(super) async fn wait_status(
    client: &reqwest::Client,
    node: &NativeNode,
    id: &str,
    status: &str,
) -> TestResult<Value> {
    let mut last = Value::Null;
    for _ in 0..300 {
        last = invocation(client, node, id).await?;
        if last["status"].as_str() == Some(status) {
            return Ok(last);
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Err(format!("native invocation {id} did not reach {status}; last={last}").into())
}

pub(super) async fn wait_sleep(
    client: &reqwest::Client,
    node: &NativeNode,
    id: &str,
) -> TestResult<Vec<Value>> {
    let mut last = Vec::new();
    for _ in 0..80 {
        last = journal(client, node, id).await?;
        if last
            .iter()
            .any(|entry| entry["entry_type"] == "Command: Sleep")
        {
            check!(!last
                .iter()
                .any(|entry| entry["entry_type"] == "Command: Output"));
            return Ok(last);
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    Err(format!("native invocation {id} did not retain Sleep; journal={last:?}").into())
}

pub(super) async fn control(
    client: &reqwest::Client,
    node: &NativeNode,
    id: &str,
    action: &str,
    evidence: &Evidence,
) -> TestResult {
    validate_id(id)?;
    check!(matches!(action, "pause" | "resume"));
    let response = client
        .patch(format!("{}/invocations/{id}/{action}", node.admin))
        .send()
        .await?;
    let status = response.status();
    let body = response.bytes().await?;
    check!(body.len() <= 1_048_576);
    evidence.write(
        &format!("{action}.json"),
        &json!({"id": id, "status": status.as_u16(), "body": String::from_utf8_lossy(&body)}),
    )?;
    let accepted = status == reqwest::StatusCode::OK
        || (action == "pause" && status == reqwest::StatusCode::ACCEPTED);
    check!(
        accepted,
        "native 1.7.10 {action} failed: {status}: {}",
        String::from_utf8_lossy(&body)
    );
    Ok(())
}

pub(super) fn prefix(actual: &[Value], retained: &[Value]) -> TestResult {
    let prefix = actual
        .get(..retained.len())
        .ok_or("native journal lost retained prefix")?;
    check!(eq; prefix, retained);
    Ok(())
}
