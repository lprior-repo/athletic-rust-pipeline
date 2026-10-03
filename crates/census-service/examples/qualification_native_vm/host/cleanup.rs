use super::{artifacts, cancellation, Process, Seed, Ssh};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::path::Path;
use std::process::ExitStatus;

pub(super) fn attach(
    result: Result<()>,
    label: &str,
    failure: Option<&anyhow::Error>,
) -> Result<()> {
    let Some(failure) = failure else {
        return result;
    };
    let context = format!("{label} failed: {failure:#}");
    match result {
        Ok(()) => Err(anyhow::anyhow!(context)),
        Err(primary) => Err(primary.context(context)),
    }
}

pub(super) fn finish(
    root: &Path,
    ssh: &Ssh,
    seed: &mut Seed,
    vm: &mut Process,
    primary: Result<()>,
) -> Result<()> {
    cancellation::cleanup(|| finish_owned(root, ssh, seed, vm, primary))
}

fn finish_owned(
    root: &Path,
    ssh: &Ssh,
    seed: &mut Seed,
    vm: &mut Process,
    primary: Result<()>,
) -> Result<()> {
    let seed_stop = seed.stop();
    let health = vm.health_status();
    let guest_stop = guest_drain(ssh, vm, health.as_ref().ok().copied().flatten());
    let vm_stop = vm.stop();
    let orderly = health.is_ok() && guest_stop.is_ok() && vm_stop.is_ok();
    let published = artifacts::publish(
        &root.join("cleanup.json"),
        &json!({"signal_reason":cancellation::current().reason(),"bootstrap":format!("{seed_stop:?}"),"guest_services":format!("{guest_stop:?}"),"guest_drain_certificate":guest_stop.as_ref().ok(),"qemu_health":format!("{health:?}"),"qemu":format!("{vm_stop:?}"),"qemu_exit":vm.evidence(),"qemu_orderly":orderly,"qemu_state":if orderly { "ORDERLY" } else { "UNPROVEN" },"failure":primary.as_ref().err().map(|error| format!("{error:#}")),"disks_and_logs_preserved":true,"serial_evidence":vm.log}),
    );
    [
        ("bootstrap cleanup", seed_stop.as_ref().err()),
        ("QEMU cleanup health", health.as_ref().err()),
        ("guest drain unproven", guest_stop.as_ref().err()),
        ("QEMU cleanup", vm_stop.as_ref().err()),
        ("cleanup evidence publication", published.as_ref().err()),
    ]
    .into_iter()
    .fold(primary, |result, (label, error)| {
        attach(result, label, error)
    })
}

fn guest_drain(ssh: &Ssh, vm: &Process, exited: Option<ExitStatus>) -> Result<Value> {
    if let Some(status) = exited {
        anyhow::bail!(
            "current guest drain cannot be proven: QEMU already exited: {status}; evidence={}",
            vm.evidence()
        );
    }
    ssh.exec("final-guest-drain", "/usr/bin/systemctl stop qualification.service && /srv/qualification/lib/ld-linux-x86-64.so.2 --library-path /srv/qualification/lib /srv/qualification/qualification guest-action --action drain-certificate")
        .and_then(|output| serde_json::from_str(output.trim()).context("guest cleanup did not return verified current drain evidence"))
}

pub(super) fn spawn_failure(root: &Path, seed: &mut Seed, error: anyhow::Error) -> Result<()> {
    cancellation::cleanup(|| {
        let stopped = seed.stop();
        let published = artifacts::publish(
            &root.join("cleanup.json"),
            &json!({"qemu_spawn_or_record_failure":true,"qemu_state":"UNKNOWN: inspect retained processes.jsonl and process-*.json reap evidence","bootstrap":format!("{stopped:?}"),"failure":format!("{error:#}"),"disks_and_logs_preserved":true}),
        );
        let result = attach(Err(error), "bootstrap cleanup", stopped.as_ref().err());
        attach(
            result,
            "cleanup evidence publication",
            published.as_ref().err(),
        )
    })
}
