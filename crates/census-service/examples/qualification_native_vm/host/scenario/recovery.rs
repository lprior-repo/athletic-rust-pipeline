use super::{artifacts, Process, Ssh};
use anyhow::Result;
use serde_json::{json, Value};
use std::path::Path;

const SHOW_STAGE: &str = "recovered-supervisor-systemd-show";
const JOURNAL_STAGE: &str = "recovered-supervisor-journal";
const SHOW_COMMAND: &str = "/usr/bin/systemctl show qualification.service --property=LoadState --property=UnitFileState --property=ActiveState --property=SubState --property=Result --property=ExecMainStatus --property=FragmentPath --property=MainPID --no-pager";
const JOURNAL_COMMAND: &str =
    "/usr/bin/journalctl -b -u qualification.service --no-pager --output=short-iso --lines=200";

#[cfg(test)]
mod tests;

pub(super) fn ready(
    ssh: &Ssh,
    root: &Path,
    before: &Value,
    boot: &Value,
    vm: &mut Process,
) -> Result<Value> {
    match ssh.action("ready") {
        Ok(recovered) => Ok(recovered),
        Err(primary) => Err(retain_failure(ssh, root, before, boot, vm, primary)),
    }
}

fn retain_failure(
    ssh: &Ssh,
    root: &Path,
    before: &Value,
    boot: &Value,
    vm: &mut Process,
    primary: anyhow::Error,
) -> anyhow::Error {
    let health = vm.health_status();
    let shown = ssh.exec(SHOW_STAGE, SHOW_COMMAND);
    let journal = ssh.exec(JOURNAL_STAGE, JOURNAL_COMMAND);
    let published = artifacts::publish(
        &root.join("recovered-supervisor-failure.json"),
        &json!({
            "failure":format!("{primary:#}"),
            "changed_boot":boot,
            "before":before,
            "qemu_health":format!("{health:?}"),
            "qemu_owner":vm.evidence(),
            "serial_evidence":vm.log,
            "systemd":diagnostic(SHOW_STAGE, SHOW_COMMAND, &shown),
            "journal":diagnostic(JOURNAL_STAGE, JOURNAL_COMMAND, &journal),
            "journal_entry_limit":200,
            "at":artifacts::now()
        }),
    );
    [
        ("recovered supervisor QEMU health", health.as_ref().err()),
        (SHOW_STAGE, shown.as_ref().err()),
        (JOURNAL_STAGE, journal.as_ref().err()),
        (
            "recovered supervisor failure evidence publication",
            published.as_ref().err(),
        ),
    ]
    .into_iter()
    .fold(primary, |primary, (label, failure)| match failure {
        Some(failure) => primary.context(format!("{label} failed: {failure:#}")),
        None => primary,
    })
}

fn diagnostic(stage: &str, command: &str, result: &Result<String>) -> Value {
    json!({
        "stage":stage,
        "command":command,
        "output":result.as_ref().ok(),
        "failure":result.as_ref().err().map(|error|format!("{error:#}"))
    })
}
