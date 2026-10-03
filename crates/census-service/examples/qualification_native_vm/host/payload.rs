use super::super::{artifacts, captures, process, transport, Host, CLOCK_WORKFLOW, WORKFLOW};
use super::file_sha;
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;

pub(super) fn prepare(config: &Host, root: &Path) -> Result<Value> {
    let payload = root.join("payload");
    std::fs::create_dir(&payload)?;
    std::fs::create_dir(payload.join("lib"))?;
    let qualification = std::env::current_exe()?;
    let binaries = [
        qualification,
        std::fs::canonicalize(&config.census_serve)?,
        std::fs::canonicalize(&config.restate)?,
    ];
    let version = process::command(
        root,
        "native-version",
        Command::new(binaries.get(2).context("Restate binary absent")?).arg("--version"),
        100,
    )?;
    ensure!(
        version
            .split_whitespace()
            .any(|part| part.starts_with("1.7.")),
        "native Restate must be actual 1.7.x: {version}"
    );
    copy_libraries(root, &payload, &binaries)?;
    let hashes = binaries
        .iter()
        .zip(["qualification", "census-serve", "restate-server"])
        .map(|(source, name)| -> Result<Value> {
            let destination = payload.join(name);
            std::fs::copy(source, &destination)?;
            Ok(json!({"binary":name,"sha256":file_sha(&destination)?}))
        })
        .collect::<Result<Vec<_>>>()?;
    let provenance = captures::prepare(&payload, &config.captures)?;
    write_native_config(&payload)?;
    let manifest = json!({"run":"vm_fixture_replay_2026_r1","season":2026,"revision":1,"cohort":2027,"ports":{"node":5122,"ingress":18095,"admin":19095,"endpoint":18096},"node":"vm-fixture-replay","cluster":"vm-fixture-replay","guest_root":"/srv/qualification","binaries":hashes,"restate_config_sha256":file_sha(&payload.join("restate.toml"))?,"native_version":version,"provenance":provenance,"operation_id":"vm_fixture_replay_2026_r1:athletes:0","workflow_keys":[WORKFLOW,CLOCK_WORKFLOW]});
    artifacts::publish(&payload.join("manifest.json"), &manifest)?;
    artifacts::publish(&root.join("manifest.json"), &manifest)?;
    Ok(manifest)
}

fn write_native_config(payload: &Path) -> Result<()> {
    let config = "roles = [\"http-ingress\", \"admin\", \"worker\", \"log-server\", \"metadata-server\"]\nnode-name = \"vm-fixture-replay\"\ncluster-name = \"vm-fixture-replay\"\nauto-provision = true\ndefault-num-partitions = 1\ndefault-replication = 1\nbase-dir = \"/srv/qualification/restate\"\nlisten-mode = \"tcp\"\nbind-ip = \"127.0.0.1\"\nbind-port = 5122\nadvertised-address = \"http://127.0.0.1:5122/\"\nshutdown-timeout = \"1m\"\ndisable-telemetry = true\nexperimental-enable-protocol-v7 = true\nexperimental-enable-vqueues = true\nexperimental-enable-scoped-virtual-objects = true\n[ingress]\nbind-address = \"127.0.0.1:18095\"\n[admin]\nbind-address = \"127.0.0.1:19095\"\n";
    artifacts::write(&payload.join("restate.toml"), config.as_bytes())
}

fn copy_libraries(root: &Path, payload: &Path, binaries: &[std::path::PathBuf]) -> Result<()> {
    let libraries = transport::libraries(root, binaries)?;
    libraries.iter().try_for_each(|source| -> Result<()> {
        let destination = payload
            .join("lib")
            .join(source.file_name().context("library filename absent")?);
        if destination.exists() {
            ensure!(
                file_sha(&destination)? == file_sha(source)?,
                "runtime library basename collision"
            );
            return Ok(());
        }
        std::fs::copy(source, destination)?;
        Ok(())
    })
}
