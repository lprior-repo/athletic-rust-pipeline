use super::super::{artifacts, host::file_sha};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::path::Path;

pub(super) fn verify_host_manifest(path: &Path) -> Result<Value> {
    let resolved =
        std::fs::canonicalize(path).with_context(|| format!("resolving {}", path.display()))?;
    let manifest = artifacts::json(&resolved)?;
    let root = resolved.parent().context("host manifest parent absent")?;
    let text = |key: &str| {
        manifest
            .get(key)
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .with_context(|| format!("host manifest {key} absent"))
    };
    let run = text("run")?;
    let node = text("node")?;
    let cluster = text("cluster")?;
    let guest_root = text("guest_root")?;
    ensure!(
        guest_root == super::super::GUEST,
        "host manifest guest_root differs"
    );
    let season = manifest
        .get("season")
        .and_then(Value::as_u64)
        .context("host manifest season absent")?;
    ensure!(season == 2026, "host manifest season differs");
    let revision = manifest
        .get("revision")
        .and_then(Value::as_u64)
        .context("host manifest revision absent")?;
    ensure!(revision == 1, "host manifest revision differs");
    let cohort = manifest
        .get("cohort")
        .and_then(Value::as_u64)
        .context("host manifest cohort absent")?;
    ensure!(cohort == 2027, "host manifest cohort differs");
    verify_ports(&manifest)?;
    verify_binaries(&manifest, &root.join("payload"))?;
    verify_config(&manifest, &root.join("payload"))?;
    let version = text("native_version")?;
    ensure!(
        version.split_whitespace().any(|part| part.contains("1.7.")),
        "native Restate must be actual 1.7.x"
    );
    let operation = text("operation_id")?;
    ensure!(!operation.is_empty(), "host manifest operation_id absent");
    let workflows = manifest
        .get("workflow_keys")
        .and_then(Value::as_array)
        .context("host manifest workflow_keys absent")?;
    ensure!(workflows.len() == 2, "host manifest workflow_keys differs");
    for workflow in workflows {
        ensure!(
            workflow.as_str().is_some_and(|value| !value.is_empty()),
            "host workflow key absent"
        );
    }
    ensure!(
        workflows.first() != workflows.get(1),
        "host workflow keys overlap"
    );
    verify_provenance(&manifest)?;
    Ok(json!({
        "manifest": resolved.display().to_string(),
        "run": run,
        "node": node,
        "cluster": cluster,
        "operation_id": operation
    }))
}

fn verify_ports(manifest: &Value) -> Result<()> {
    let ports = manifest
        .get("ports")
        .context("host manifest ports absent")?;
    let port = |key: &str| {
        ports
            .get(key)
            .and_then(Value::as_u64)
            .filter(|value| (1..=65535).contains(value))
            .with_context(|| format!("host manifest port {key} absent"))
    };
    let node = port("node")?;
    let ingress = port("ingress")?;
    let admin = port("admin")?;
    let endpoint = port("endpoint")?;
    ensure!(
        node != ingress
            && node != admin
            && node != endpoint
            && ingress != admin
            && ingress != endpoint
            && admin != endpoint,
        "host manifest ports overlap"
    );
    Ok(())
}

fn verify_binaries(manifest: &Value, payload: &Path) -> Result<()> {
    let binaries = manifest
        .get("binaries")
        .and_then(Value::as_array)
        .context("host manifest binaries absent")?;
    ensure!(binaries.len() == 3, "host native binary set differs");
    for name in ["qualification", "census-serve", "restate-server"] {
        let mut matching = binaries
            .iter()
            .filter(|binary| binary.get("binary").and_then(Value::as_str) == Some(name));
        let binary = matching
            .next()
            .with_context(|| format!("host native binary {name} absent"))?;
        ensure!(matching.next().is_none(), "host native binary duplicated");
        let digest = binary
            .get("sha256")
            .and_then(Value::as_str)
            .context("binary digest absent")?;
        ensure!(is_hex64(digest), "binary digest is not hex64");
        let observed = file_sha(&payload.join(name))?;
        ensure!(observed.as_str() == digest, "staged native binary differs");
    }
    Ok(())
}

fn verify_config(manifest: &Value, payload: &Path) -> Result<()> {
    let digest = manifest
        .get("restate_config_sha256")
        .and_then(Value::as_str)
        .context("host restate config digest absent")?;
    ensure!(is_hex64(digest), "restate config digest is not hex64");
    let observed = file_sha(&payload.join("restate.toml"))?;
    ensure!(
        observed.as_str() == digest,
        "staged native configuration differs"
    );
    Ok(())
}

fn verify_provenance(manifest: &Value) -> Result<()> {
    let provenance = manifest
        .get("provenance")
        .context("host manifest provenance absent")?;
    ensure!(
        provenance.get("fresh_public_acquisition") == Some(&json!(false)),
        "host provenance freshness differs"
    );
    let transport = provenance
        .get("transport")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .context("host provenance transport absent")?;
    ensure!(!transport.is_empty(), "host provenance transport absent");
    let semantics = provenance
        .get("date_semantics")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .context("host provenance date semantics absent")?;
    ensure!(
        semantics.contains("immutable"),
        "host provenance date semantics differs"
    );
    Ok(())
}

fn is_hex64(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests;
