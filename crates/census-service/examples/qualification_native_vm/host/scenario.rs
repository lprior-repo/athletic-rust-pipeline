use super::super::{artifacts, oracle, process::Process, qmp, transport::Ssh};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::path::Path;
use std::time::Duration;

#[cfg(test)]
use clock::{cross_midnight, crossed, validate_clock_baseline};
use clock::{midnight, offline_snapshot};
mod clock;
mod clock_window;
mod recovery;

pub(super) fn provision_and_exercise(
    ssh: &Ssh,
    root: &Path,
    manifest: &Value,
    vm: &mut Process,
) -> Result<()> {
    let boot = ssh.wait_boot(None, vm)?;
    artifacts::publish(&root.join("first-boot.json"), &boot)?;
    copy_payload(ssh, root)?;
    ssh.exec("guest-initialize", "/root/qualification-payload/lib/ld-linux-x86-64.so.2 --library-path /root/qualification-payload/lib /root/qualification-payload/qualification guest-init")?;
    let ready = ssh.action("ready")?;
    ensure!(
        ready.get("manifest") == Some(manifest),
        "guest recovery config differs from host manifest"
    );
    let deployment = ssh.action("register")?;
    artifacts::publish(&root.join("deployment.json"), &deployment)?;
    let ack = ssh.action("ingest-first")?;
    artifacts::publish(&root.join("host-acknowledgement.json"), &ack)?;
    let active = ssh.action("sweep-reboot-start")?;
    artifacts::publish(&root.join("host-active-before-reset.json"), &active)?;
    let source = ssh.action("jurisdiction-reboot-start")?;
    artifacts::publish(&root.join("host-source-active-before-reset.json"), &source)?;
    reboot(ssh, root, &ready, &deployment, vm)?;
    let physical_reboot = offline_snapshot(ssh, root, "after-reboot")?;
    ssh.exec(
        "restart-after-reboot",
        "/usr/bin/systemctl start qualification.service",
    )?;
    wait_ready_after_restart(ssh, root)?;
    let quiescent = ssh.action("jurisdiction-quiescence")?;
    artifacts::publish(
        &root.join("source-quiescent-before-midnight.json"),
        &quiescent,
    )?;
    let physical_clock = midnight(ssh, root)?;
    let reconciled = oracle::reconcile(&ack, &physical_reboot, &physical_clock)?;
    oracle::reconcile_source(
        physical_reboot
            .get("source_recovery")
            .context("cold reboot source oracle absent")?,
        physical_clock
            .get("source_recovery")
            .context("cold midnight source oracle absent")?,
    )?;
    artifacts::publish(&root.join("measured-durability-oracles.json"), &reconciled)
}

fn reboot(
    ssh: &Ssh,
    root: &Path,
    before: &Value,
    deployment: &Value,
    vm: &mut Process,
) -> Result<()> {
    let old = before
        .get("clock")
        .and_then(|value| value.get("boot_id"))
        .and_then(Value::as_str)
        .context("original boot_id absent")?;
    let boundary_at = artifacts::now();
    let reset = qmp::reset(root)?;
    artifacts::publish(
        &root.join("host-reset-boundary-order.json"),
        &json!({"host_boundary_received_at":boundary_at,"qmp_reset":reset,
            "boundary_artifact":"host-source-active-before-reset.json"}),
    )?;
    let boot = ssh.wait_boot(Some(old), vm)?;
    let recovered = recovery::ready(ssh, root, before, &boot, vm)?;
    let clock = recovered.get("clock").context("recovered clock absent")?;
    ensure!(
        clock.get("boot_id").and_then(Value::as_str) != Some(old),
        "hard reset did not change actual guest boot_id"
    );
    ensure!(
        clock.get("machine_id")
            == before
                .get("clock")
                .and_then(|value| value.get("machine_id")),
        "guest machine identity replaced"
    );
    let saved_id = deployment.get("id").context("deployment ID absent")?;
    ensure!(
        serde_json::to_string(
            recovered
                .get("deployments")
                .context("recovered deployments absent")?
        )?
        .contains(saved_id.as_str().context("deployment ID invalid")?),
        "same native deployment did not recover"
    );
    let repeat = ssh.action("repeat-reboot")?;
    let workflow = ssh.action("sweep-reboot-finish");
    let source = ssh.action("jurisdiction-reboot-finish");
    artifacts::publish(
        &root.join("reboot-recovery-outcomes.json"),
        &json!({
            "sweep": workflow.as_ref().map_err(|error| format!("{error:#}")),
            "source": source.as_ref().map_err(|error| format!("{error:#}")),
            "scope": "both original recovery checks attempted before propagating either failure"
        }),
    )?;
    let workflow = workflow?;
    let source = source?;
    artifacts::publish(
        &root.join("host-source-recovered-after-reset.json"),
        &source,
    )?;
    artifacts::publish(
        &root.join("reboot-oracle.json"),
        &json!({"verdict":"PASS","changed_boot":boot,"before":before,"after":recovered,"replayed_effect":repeat,"same_active_sweep_recovery":workflow,"same_active_source_recovery":source,"limit":"source-stage witness is not a physical HTTP/response/capture/parse subphase witness"}),
    )
}

fn copy_payload(ssh: &Ssh, root: &Path) -> Result<()> {
    ssh.exec(
        "staging-directory",
        "/usr/bin/mkdir -p /root/qualification-payload/lib",
    )?;
    let payload = root.join("payload");
    let entries = std::fs::read_dir(&payload)?
        .take(129)
        .collect::<std::io::Result<Vec<_>>>()?;
    ensure!(entries.len() <= 128, "payload file budget exceeded");
    let files = entries
        .into_iter()
        .filter_map(|entry| entry.path().is_file().then(|| entry.path()))
        .collect::<Vec<_>>();
    ssh.copy("payload-copy", &files, "/root/qualification-payload/")?;
    let libraries = std::fs::read_dir(payload.join("lib"))?
        .take(129)
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    ensure!(libraries.len() <= 128, "runtime library budget exceeded");
    ssh.copy(
        "libraries-copy",
        &libraries,
        "/root/qualification-payload/lib/",
    )
}

fn wait_ready_after_restart(ssh: &Ssh, root: &Path) -> Result<()> {
    const SHOW: &str = "restart-supervisor-systemd-show";
    const JOURNAL: &str = "restart-supervisor-journal";
    const SHOW_COMMAND: &str =
        "/usr/bin/systemctl show qualification.service --property=LoadState --property=UnitFileState --property=ActiveState --property=SubState --property=Result --property=ExecMainStatus --property=FragmentPath --property=MainPID --no-pager";
    const JOURNAL_COMMAND: &str =
        "/usr/bin/journalctl -b -u qualification.service --no-pager --output=short-iso --lines=200";
    for attempt in 0..120 {
        match ssh.action("ready") {
            Ok(_) => return Ok(()),
            Err(_) if attempt < 119 => ssh.pause(Duration::from_secs(1))?,
            Err(last) => {
                let shown = ssh.exec(SHOW, SHOW_COMMAND);
                let journal = ssh.exec(JOURNAL, JOURNAL_COMMAND);
                let shown_failure = shown.as_ref().err().map(|e| format!("{e:#}"));
                let journal_failure = journal.as_ref().err().map(|e| format!("{e:#}"));
                let published = artifacts::publish(
                    &root.join("restart-readiness.json"),
                    &json!({
                        "failure": format!("{last:#}"),
                        "attempts": 120,
                        "systemd": {
                            "stage": SHOW,
                            "command": SHOW_COMMAND,
                            "output": shown.as_ref().ok(),
                            "failure": shown_failure
                        },
                        "journal": {
                            "stage": JOURNAL,
                            "command": JOURNAL_COMMAND,
                            "output": journal.as_ref().ok(),
                            "failure": journal_failure
                        },
                        "journal_entry_limit": 200,
                        "at": artifacts::now()
                    }),
                );
                return published
                    .map_err(|e| anyhow::anyhow!("restart readiness publication failed: {e:#}"))
                    .and_then(|_| Err(last));
            }
        }
    }
    anyhow::bail!("restart readiness probe budget exhausted")
}
#[cfg(test)]
mod tests;
