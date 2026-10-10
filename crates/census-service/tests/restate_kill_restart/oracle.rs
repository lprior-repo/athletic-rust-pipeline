use super::readback::{invocation_verdict, require_reply_body, single_invocation_row, Verdict};
use crate::{Endpoint, Node, POLL_INTERVAL};
use serde_json::Value;
use std::time::{Duration, Instant};

async fn admin_query(
    client: &reqwest::Client,
    node: &Node,
    query: String,
) -> Result<Value, String> {
    let response = client
        .post(format!("{}query", node.admin))
        .header("accept", "application/json")
        .json(&serde_json::json!({ "query": query }))
        .send()
        .await
        .map_err(|error| error.to_string())?;
    let status = response.status();
    let text = response.text().await.map_err(|error| error.to_string())?;
    if !status.is_success() {
        return Err(format!("the admin query answered {status}: {text}"));
    }
    serde_json::from_str(&text).map_err(|error| format!("{error}: {text}"))
}

async fn invocation_row(
    client: &reqwest::Client,
    node: &Node,
    invocation: &str,
) -> Result<Value, String> {
    let observation = admin_query(
        client,
        node,
        format!(
            "SELECT id, status, completion_result, completion_failure FROM sys_invocation \
             WHERE id = '{invocation}'"
        ),
    )
    .await?;
    let rows = observation
        .get("rows")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("no rows in the admin's answer: {observation}"))?;
    single_invocation_row(rows, invocation).cloned()
}

fn entry_label(row: &Value) -> String {
    let index = row
        .get("index")
        .map_or_else(|| String::from("?"), |value| value.to_string());
    let kind = row
        .get("entry_type")
        .and_then(Value::as_str)
        .map_or("?", core::convert::identity);
    format!("{index}:{kind}")
}

async fn journal_digest(client: &reqwest::Client, node: &Node, invocation: &str) -> String {
    let query = format!(
        "SELECT index, entry_type FROM sys_journal WHERE id = '{invocation}' ORDER BY index"
    );
    match admin_query(client, node, query).await {
        Ok(observation) => match observation.get("rows").and_then(Value::as_array) {
            Some(rows) => {
                let mut entries: Vec<String> = rows.iter().map(entry_label).collect();
                let total = entries.len();
                if entries.len() > 8 {
                    entries.drain(..entries.len() - 8);
                }
                format!("{} ({total} entries)", entries.join(", "))
            }
            None => format!("the journal answer holds no rows: {observation}"),
        },
        Err(error) => format!("the journal is unreadable: {error}"),
    }
}

pub(super) async fn require_completed_success(
    client: &reqwest::Client,
    node: &Node,
    invocation: &str,
    deadline: Instant,
) -> Result<Value, String> {
    loop {
        let row = invocation_row(client, node, invocation).await?;
        match invocation_verdict(&row, invocation) {
            Verdict::Succeeded => return Ok(row),
            Verdict::Refused(reason) => {
                return Err(format!(
                    "the original invocation {invocation} did not recover to a completed success: \
                     {reason}; observation={row}; journal={}",
                    journal_digest(client, node, invocation).await
                ));
            }
            Verdict::Pending(reason) => {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return Err(format!(
                        "the original invocation {invocation} never reached a completed success \
                         within the bounded recovery budget: {reason}; observation={row}; \
                         journal={}",
                        journal_digest(client, node, invocation).await
                    ));
                }
                tokio::time::sleep(POLL_INTERVAL.min(remaining)).await;
            }
        }
    }
}

pub(super) async fn attached_output(
    client: &reqwest::Client,
    node: &Node,
    invocation: &str,
    budget: Duration,
) -> Result<Value, String> {
    let attach = client
        .get(format!(
            "{}restate/invocation/{invocation}/attach",
            node.ingress
        ))
        .send();
    let response = tokio::time::timeout(budget, attach)
        .await
        .map_err(|_| format!("attaching to {invocation} exceeded {budget:?}"))?
        .map_err(|error| error.to_string())?;
    let status = response.status();
    let text = response.text().await.map_err(|error| error.to_string())?;
    if !status.is_success() {
        return Err(format!(
            "attaching to {invocation} answered {status}: {text}"
        ));
    }
    let body: Value = serde_json::from_str(&text).map_err(|error| {
        format!("the attached output of {invocation} is not json: {error}: {text}")
    })?;
    require_reply_body(&body)?;
    Ok(body)
}

pub(super) fn failure_logs(endpoint: &Endpoint, node: &Node) -> String {
    format!(
        "--- endpoint log ({}):\n{}\n--- node log ({}):\n{}",
        endpoint.log_path.display(),
        endpoint.log(),
        node.log_path.display(),
        std::fs::read_to_string(&node.log_path).map_or(Default::default(), std::convert::identity),
    )
}

pub(super) fn require_mid_merge_progress(reached: usize, all: usize) -> Result<(), String> {
    if reached < 1 {
        return Err(format!(
            "the endpoint never reached its merge phase before the kill: no table snapshot was \
             written (0 of {all})"
        ));
    }
    if reached >= all {
        return Err(format!(
            "the merge had already written every one of its {all} snapshots before the kill \
             landed, so this run never had to resume"
        ));
    }
    Ok(())
}
