use super::{artifacts, bootstrap::Seed, cancellation, process::Process, transport::Ssh, Host};
use anyhow::{bail, ensure, Context, Result};
use serde_json::json;
use std::path::Path;

mod cleanup;
mod machine;
mod payload;
mod scenario;

pub fn run(config: Host) -> Result<()> {
    std::fs::create_dir(&config.root)
        .context("fresh exclusive VM root required; recovery artifacts never overwritten")?;
    let root = std::fs::canonicalize(&config.root)?;
    let mut signals = cancellation::Signals::install()?;
    let result = execute(&config, &root).and_then(|()| cancellation::checkpoint());
    let signal_reason = signals.reason();
    let joined = signals.finish();
    let failure = match &result {
        Ok(()) => None,
        Err(error) => Some(format!("{error:#}")),
    };
    let cleanup_record = if !root.join("cleanup.json").exists() {
        artifacts::publish(
            &root.join("cleanup.json"),
            &json!({"qemu_state":"UNKNOWN: execution did not publish final cleanup; inspect retained process evidence","signal_reason":signal_reason,"failure":failure,"disks_and_logs_preserved":true}),
        )
    } else {
        Ok(())
    };
    let verdict_record = artifacts::publish(
        &root.join("verdict.json"),
        &json!({"verdict":"BLOCKED_OR_UNPROVEN","failure":failure,"signal_reason":signal_reason,"signal_worker":format!("{joined:?}"),"model":"openai-codex/gpt-6.1-sol","missing_required_obligations":["same active JurisdictionCensus/source-stage recovery boundary across guest reboot", "successful production Fetcher acquisitions on both sides of guest midnight with honest new fetched_at"],"scope":"fixture transport/replay native VM qualification; not national completion or full scenario03/scenario12 PASS"}),
    );
    [
        (
            "fallback cleanup evidence publication",
            cleanup_record.as_ref().err(),
        ),
        ("verdict publication", verdict_record.as_ref().err()),
        ("signal worker finalization", joined.as_ref().err()),
    ]
    .into_iter()
    .fold(result, |result, (label, error)| {
        cleanup::attach(result, label, error)
    })?;
    bail!(
        "BLOCKED_OR_UNPROVEN: measured VM/Sweep/Ingest/reused-capture oracles completed, but full census/source reboot and fresh acquisition clock obligations remain unproven; see retained verdict.json"
    )
}

fn execute(config: &Host, root: &Path) -> Result<()> {
    machine::qmp_socket(root)?;
    let manifest = payload::prepare(config, root)?;
    let reservation = std::net::TcpListener::bind("127.0.0.1:0")?;
    let ssh_port = reservation.local_addr()?.port();
    let ssh = Ssh {
        root: root.into(),
        port: ssh_port,
    };
    let key = machine::key(root)?;
    machine::create_disks(config, root)?;
    let mut seed = Seed::start(root, key.trim())?;
    let spawned = machine::qemu(config, root, ssh_port, seed.port).and_then(|mut command| {
        drop(reservation);
        Process::spawn(root, "qemu", &mut command)
    });
    let mut vm = match spawned {
        Ok(vm) => vm,
        Err(error) => return cleanup::spawn_failure(root, &mut seed, error),
    };
    let scenario = scenario::provision_and_exercise(&ssh, root, &manifest, &mut vm);
    cleanup::finish(root, &ssh, &mut seed, &mut vm, scenario)
}

pub(super) fn file_sha(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    ensure!(
        file.metadata()?.len() <= 512 * 1024 * 1024,
        "binary exceeds 512 MiB qualification budget"
    );
    let mut buffer = [0_u8; 65536];
    let mut digest = Sha256::new();
    (0..8193)
        .find_map(|_| match file.read(&mut buffer) {
            Ok(0) => Some(Ok(())),
            Ok(count) => match buffer.get(..count) {
                Some(bytes) => {
                    digest.update(bytes);
                    None
                }
                None => Some(Err(anyhow::anyhow!("binary read exceeded buffer"))),
            },
            Err(error) => Some(Err(error.into())),
        })
        .context("binary hash exceeded fixed read budget")??;
    Ok(format!("{:x}", digest.finalize()))
}
