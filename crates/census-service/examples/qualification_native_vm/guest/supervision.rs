use super::super::{
    artifacts, cancellation, drain,
    process::{self, Process},
    GUEST,
};
use super::{launch, verify_recovery_inputs};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::path::Path;
use std::process::ExitStatus;
use std::time::Duration;

pub(super) fn run() -> Result<()> {
    let mut signals = cancellation::Signals::install()?;
    let result = supervise(&signals);
    let joined = signals.finish();
    joined?;
    result
}

fn supervise(signals: &cancellation::Signals) -> Result<()> {
    let root = Path::new(GUEST);
    private_root(root)?;
    let boot = std::fs::read_to_string("/proc/sys/kernel/random/boot_id")?;
    let generation = process::generation()?;
    let starting = json!({"boot_id":boot.trim(),"supervisor_pid":std::process::id(),"generation":generation,"phase":"starting"});
    artifacts::publish(&root.join("current-owner.json"), &starting)?;
    let manifest = artifacts::json(&root.join("manifest.json"))?;
    verify_recovery_inputs(&manifest)?;
    let mut node_command = launch("restate-server");
    node_command.env_remove("CENSUS_NATIVE_SOURCE_BOUNDARY");
    node_command.args([
        "--no-logo",
        "--config-file",
        &format!("{GUEST}/restate.toml"),
    ]);
    let mut node = Process::spawn(root, "restate", &mut node_command)?;
    let mut endpoint_command = launch("census-serve");
    endpoint_command.env(
        "CENSUS_NATIVE_SOURCE_BOUNDARY",
        "/srv/qualification/native-source-boundary-config.json",
    );
    endpoint_command.args([
        "--listen",
        "127.0.0.1:18096",
        "--data-dir",
        &format!("{GUEST}/store"),
        "--max-concurrent",
        "2",
        "--drain-timeout",
        "30",
    ]);
    let mut endpoint = Process::spawn(root, "serve", &mut endpoint_command)?;
    let identity = json!({"boot_id":boot.trim(),"supervisor_pid":std::process::id(),"generation":generation,"endpoint":endpoint.identity(),"node":node.identity()});
    artifacts::publish(&root.join("current-owner.json"), &identity)?;
    artifacts::append(
        &root.join("boots.jsonl"),
        &json!({"identity":identity,"machine_id":std::fs::read_to_string("/etc/machine-id")?.trim(),"disk":artifacts::json(&root.join("disk.json"))?,"manifest_sha256":artifacts::sha(&serde_json::to_vec(&manifest)?),"at":artifacts::now()}),
    )?;
    let waited = wait_shutdown(&mut endpoint, &mut node, signals);
    let endpoint_stop = endpoint.stop();
    let node_stop = node.stop();
    let drain = certify(
        root,
        &identity,
        &endpoint,
        &node,
        &waited,
        &endpoint_stop,
        &node_stop,
    );
    drain?;
    endpoint_stop?;
    node_stop?;
    waited
}

fn private_root(root: &Path) -> Result<()> {
    use std::os::unix::fs::MetadataExt;
    let metadata = std::fs::symlink_metadata(root)?;
    ensure!(
        metadata.is_dir()
            && metadata.uid() == std::fs::metadata("/proc/self")?.uid()
            && metadata.mode() & 0o777 == 0o700,
        "qualification root must be the supervisor-owned private directory"
    );
    Ok(())
}

fn certify(
    root: &Path,
    identity: &Value,
    endpoint: &Process,
    node: &Process,
    waited: &Result<()>,
    endpoint_stop: &Result<Value>,
    node_stop: &Result<Value>,
) -> Result<()> {
    let log = artifacts::read(&endpoint.log);
    let counts = log
        .as_ref()
        .map_err(|error| anyhow::anyhow!("{error:#}"))
        .and_then(|bytes| drain::parse(std::str::from_utf8(bytes)?));
    let orderly = waited.is_ok()
        && endpoint_stop.is_ok()
        && node_stop.is_ok()
        && counts.is_ok()
        && endpoint
            .evidence()
            .get("exit")
            .and_then(|exit| exit.get("code"))
            .and_then(Value::as_i64)
            == Some(0);
    let evidence = json!({"identity":identity,"orderly":orderly,"wait":format!("{waited:?}"),"endpoint_stop":format!("{endpoint_stop:?}"),"node_stop":format!("{node_stop:?}"),"endpoint_exit":endpoint.evidence(),"node_exit":node.evidence(),"counts":counts.as_ref().ok(),"count_failure":counts.as_ref().err().map(|error|format!("{error:#}")),"log_sha256":log.as_ref().ok().map(|bytes|artifacts::sha(bytes)),"at":artifacts::now()});
    let generation = identity
        .get("generation")
        .and_then(Value::as_str)
        .context("supervisor generation absent")?;
    artifacts::publish(&root.join(format!("drain-{generation}.json")), &evidence)?;
    artifacts::publish(&root.join("current-drain.json"), &evidence)?;
    ensure!(orderly, "current supervisor drain failed: {evidence}");
    Ok(())
}

fn wait_shutdown(
    endpoint: &mut Process,
    node: &mut Process,
    signals: &cancellation::Signals,
) -> Result<()> {
    (0..36_000)
        .find_map(|_| {
            let step = shutdown_step(
                || endpoint.health_status(),
                || node.health_status(),
                || signals.reason(),
            );
            match step {
                Ok(true) => return Some(Ok(())),
                Err(error) => return Some(Err(error)),
                Ok(false) => {}
            }
            std::thread::sleep(Duration::from_millis(100));
            None
        })
        .context("guest supervisor one-hour qualification deadline expired")?
}

fn shutdown_step(
    endpoint: impl FnOnce() -> Result<Option<ExitStatus>>,
    node: impl FnOnce() -> Result<Option<ExitStatus>>,
    mut reason: impl FnMut() -> i32,
) -> Result<bool> {
    let initial = reason();
    ensure!(
        matches!(initial, 0 | 15 | 2),
        "guest supervisor cancelled by resource/deadline reason={initial}"
    );
    let endpoint_health = endpoint();
    let node_health = node();
    let endpoint_status = endpoint_health
        .map_err(|error| error.context(format!("independent node health={node_health:?}")))?;
    let node_status = node_health?;
    let current = reason();
    ensure!(
        matches!(current, 0 | 15 | 2),
        "guest supervisor cancelled by resource/deadline reason={current}"
    );
    let stopping = matches!(initial, 15 | 2) || matches!(current, 15 | 2);
    ensure!(
        endpoint_status.is_none()
            || (stopping && endpoint_status.is_some_and(|status| status.success())),
        "endpoint exited unexpectedly: {endpoint_status:?}"
    );
    ensure!(
        node_status.is_none() || (stopping && node_status.is_some_and(|status| status.success())),
        "node exited unexpectedly: {node_status:?}"
    );
    Ok(stopping)
}

#[cfg(test)]
mod tests;
