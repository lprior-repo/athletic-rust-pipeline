use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::path::Path;

use super::artifacts::{read_bounded, write_json};
use super::config::Config;
use super::process::OwnedProcess;
use super::proxy::RefusalProxy;

#[cfg(test)]
mod tests;

struct ProcessCleanup {
    endpoint: Result<Value>,
    certificate: Result<Value>,
    node: Result<Value>,
    evidence: Value,
}

fn stop_children(endpoint: &mut OwnedProcess, node: &mut OwnedProcess) -> ProcessCleanup {
    let endpoint_result = endpoint.stop();
    let certificate = drain_certificate(&endpoint.log);
    let node_result = node.stop();
    let evidence = json!({
        "endpoint_term_reap":endpoint.evidence(),
        "endpoint_error":endpoint_result.as_ref().err().map(|error| format!("{error:#}")),
        "drain_certificate":certificate.as_ref().ok(),
        "drain_error":certificate.as_ref().err().map(|error| format!("{error:#}")),
        "node_term_reap":node.evidence(),
        "node_error":node_result.as_ref().err().map(|error| format!("{error:#}")),
    });
    ProcessCleanup {
        endpoint: endpoint_result,
        certificate,
        node: node_result,
        evidence,
    }
}

pub fn cleanup(
    config: &Config,
    endpoint: &mut OwnedProcess,
    node: &mut OwnedProcess,
    proxy: &mut RefusalProxy,
) -> Result<Value> {
    let processes = stop_children(endpoint, node);
    let proxy_result = proxy.stop();
    let ports = config.ports.iter().map(|port| {
        let bound = std::net::TcpListener::bind(("127.0.0.1", *port));
        json!({"port":port, "bind_succeeded_after_reap":bound.is_ok(), "error":bound.err().map(|error| error.to_string())})
    }).collect::<Vec<_>>();
    let mut evidence = processes.evidence;
    let object = evidence
        .as_object_mut()
        .context("cleanup evidence is not an object")?;
    object.insert("proxy".into(), json!(proxy_result.as_ref().ok()));
    object.insert(
        "proxy_error".into(),
        json!(proxy_result.as_ref().err().map(ToString::to_string)),
    );
    object.insert("native_and_endpoint_ports".into(), json!(ports));
    object.insert("artifacts_preserved".into(), json!(true));
    object.insert("sigkill_used".into(), json!(false));
    write_json(&config.root.join("cleanup.json"), &evidence)?;
    let endpoint_value = processes.endpoint?;
    processes.certificate?;
    let node_value = processes.node?;
    proxy_result?;
    ensure!(
        endpoint_value.get("success").and_then(Value::as_bool) == Some(true)
            && node_value.get("success").and_then(Value::as_bool) == Some(true),
        "owned endpoint or node exited unsuccessfully during cleanup"
    );
    ensure!(
        ports.iter().all(|port| port
            .get("bind_succeeded_after_reap")
            .and_then(Value::as_bool)
            == Some(true)),
        "owned endpoint/native ports remain bound after reaping"
    );
    Ok(evidence)
}

pub fn stop_endpoint(endpoint: &mut OwnedProcess) -> Result<Value> {
    let reaped = endpoint.stop()?;
    let certificate = drain_certificate(&endpoint.log)?;
    ensure!(
        reaped.get("success").and_then(Value::as_bool) == Some(true),
        "interruption endpoint exited unsuccessfully"
    );
    Ok(
        json!({"endpoint_term_reap":reaped, "drain_certificate":certificate,
        "reaped_at":super::artifacts::now()?, "sigkill_used":false, "artifacts_preserved":true}),
    )
}

fn drain_certificate(log: &Path) -> Result<Value> {
    let text = read_bounded(log)?;
    let line = text
        .lines()
        .find(|line| line.starts_with("drained: accepted="))
        .context("endpoint exited without an observed drain certificate")?;
    let numbers = line
        .split_whitespace()
        .skip(1)
        .map(|field| -> Result<_> {
            let (name, value) = field.split_once('=').context("malformed drain field")?;
            Ok((name, value.parse::<u64>()?))
        })
        .collect::<Result<std::collections::BTreeMap<_, _>>>()?;
    let accepted = numbers
        .get("accepted")
        .context("accepted drain count missing")?;
    let completed = numbers
        .get("completed")
        .context("completed drain count missing")?;
    let terminal = ["completed", "cancelled", "timed_out", "aborted", "panicked"]
        .iter()
        .try_fold(0_u64, |total, name| -> Result<_> {
            total
                .checked_add(*numbers.get(name).context("terminal drain count missing")?)
                .context("drain count overflow")
        })?;
    ensure!(
        *accepted == terminal && *completed <= *accepted,
        "drain accounting is incomplete"
    );
    ensure!(
        ["timed_out", "aborted", "panicked"]
            .iter()
            .all(|name| numbers.get(name) == Some(&0)),
        "endpoint drain retained failed work"
    );
    Ok(json!({"line":line, "counts":numbers, "balanced":true}))
}
