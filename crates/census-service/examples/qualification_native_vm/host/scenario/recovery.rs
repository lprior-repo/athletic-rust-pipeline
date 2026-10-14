use super::{artifacts, Process, Ssh};
use anyhow::Result;
use serde_json::{json, Map, Value};
use std::path::Path;

const SHOW_STAGE: &str = "recovered-supervisor-systemd-show";
const JOURNAL_STAGE: &str = "recovered-supervisor-journal";
const SHOW_COMMAND: &str = "/usr/bin/systemctl show qualification.service --property=LoadState --property=UnitFileState --property=ActiveState --property=SubState --property=Result --property=ExecMainStatus --property=FragmentPath --property=MainPID --no-pager";
const JOURNAL_COMMAND: &str =
    "/usr/bin/journalctl -b -u qualification.service --no-pager --output=short-iso --lines=200";

#[cfg(test)]
mod tests;

enum FailureScope {
    RecoveredReadiness,
    OfflineSnapshot,
}

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
    let fields = [
        ("phase", json!("recovered-readiness")),
        ("changed_boot", boot.clone()),
        ("before", before.clone()),
        ("qemu_health", json!(format!("{health:?}"))),
        ("qemu_owner", vm.evidence()),
        ("serial_evidence", json!(vm.log)),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_owned(), value))
    .collect();
    let failure = retain_diagnostics(
        ssh,
        &root.join("recovered-supervisor-failure.json"),
        fields,
        primary,
        FailureScope::RecoveredReadiness,
    );
    append_failure(
        failure,
        "recovered supervisor QEMU health",
        health.as_ref().err(),
    )
}

pub(super) fn offline_failure(
    ssh: &Ssh,
    root: &Path,
    label: &str,
    phase: &str,
    primary: anyhow::Error,
) -> anyhow::Error {
    let fields = [("phase", json!(phase)), ("label", json!(label))]
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect();
    retain_diagnostics(
        ssh,
        &root.join(format!("{label}-offline-failure.json")),
        fields,
        primary,
        FailureScope::OfflineSnapshot,
    )
}

fn retain_diagnostics(
    ssh: &Ssh,
    path: &Path,
    mut fields: Map<String, Value>,
    primary: anyhow::Error,
    scope: FailureScope,
) -> anyhow::Error {
    let (show_stage, journal_stage, publication_label) = match scope {
        FailureScope::RecoveredReadiness => (
            SHOW_STAGE,
            JOURNAL_STAGE,
            "recovered supervisor failure evidence publication",
        ),
        FailureScope::OfflineSnapshot => (
            "offline-supervisor-systemd-show",
            "offline-supervisor-journal",
            "offline failure evidence publication",
        ),
    };
    let shown = ssh.exec(show_stage, SHOW_COMMAND);
    let journal = ssh.exec(journal_stage, JOURNAL_COMMAND);
    fields.extend(
        [
            ("verdict", json!("UNPROVEN")),
            ("failure", json!(format!("{primary:#}"))),
            ("systemd", diagnostic(show_stage, SHOW_COMMAND, &shown)),
            (
                "journal",
                diagnostic(journal_stage, JOURNAL_COMMAND, &journal),
            ),
            ("journal_entry_limit", json!(200)),
            ("command_capture_limit_bytes", json!(artifacts::LIMIT)),
            ("at", json!(artifacts::now())),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value)),
    );
    let published = artifacts::publish(path, &fields);
    [
        (show_stage, shown.as_ref().err()),
        (journal_stage, journal.as_ref().err()),
        (publication_label, published.as_ref().err()),
    ]
    .into_iter()
    .fold(primary, |primary, (label, failure)| {
        append_failure(primary, label, failure)
    })
}

fn append_failure(
    primary: anyhow::Error,
    label: &str,
    failure: Option<&anyhow::Error>,
) -> anyhow::Error {
    match failure {
        Some(failure) => primary.context(format!("{label} failed: {failure:#}")),
        None => primary,
    }
}

fn diagnostic(stage: &str, command: &str, result: &Result<String>) -> Value {
    json!({
        "stage":stage,
        "command":command,
        "output":result.as_ref().ok(),
        "failure":result.as_ref().err().map(|error|format!("{error:#}"))
    })
}
