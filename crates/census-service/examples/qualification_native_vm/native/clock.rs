use super::super::{artifacts, process, GUEST};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::path::Path;

#[cfg(test)]
mod tests;

pub(super) fn set_clock() -> Result<Value> {
    let root = Path::new(GUEST);
    process::command(
        root,
        "guest-clock-stop-sync",
        std::process::Command::new("/usr/bin/systemctl").args([
            "stop",
            "systemd-timesyncd.service",
            "systemd-time-wait-sync.service",
        ]),
        100,
    )?;
    synchronization_masked(root)?;
    let before = clock()?;
    let date = before
        .get("date")
        .and_then(Value::as_str)
        .context("guest date absent")?;
    let command_stdout = process::command(
        root,
        "guest-clock-set",
        std::process::Command::new("/usr/bin/date").args([
            "-u",
            "--set",
            &format!("{date} 23:55:00 UTC"),
        ]),
        100,
    )?;
    let after = clock()?;
    let validation = validate_set_clock(&before, &after, date);
    let diagnostics = diagnostic_snapshot(root);
    let evidence = json!({
        "before": before,
        "after": after,
        "command_stdout": command_stdout,
        "target_utc": format!("{date}T23:55:00Z"),
        "validation_error": validation.as_ref().err().map(|error| format!("{error:#}")),
        "diagnostics": diagnostics,
        "pre_midnight_lead_seconds": 300,
        "crossing_budget_seconds": 330,
        "injection": "owned guest CLOCK_REALTIME only; natural midnight passage",
        "public_freshness_claim": false
    });
    artifacts::publish(
        &root.join("guest-clock-injection-validation.json"),
        &evidence,
    )?;
    validation?;
    Ok(evidence)
}

fn diagnostic_snapshot(root: &Path) -> Value {
    let external_realtime = diagnostic_probe(
        root,
        "guest-clock-observed-date",
        std::process::Command::new("/usr/bin/date").args(["-u", "--iso-8601=ns"]),
    );
    let running_services = diagnostic_probe(
        root,
        "guest-clock-running-services",
        std::process::Command::new("/usr/bin/systemctl").args([
            "list-units",
            "--type=service",
            "--state=running",
            "--no-legend",
            "--no-pager",
        ]),
    );
    json!({"external_realtime": external_realtime, "running_services": running_services})
}

fn diagnostic_probe(root: &Path, label: &str, command: &mut std::process::Command) -> Value {
    match process::command(root, label, command, 100) {
        Ok(stdout) => json!({"status": "completed", "stdout": stdout}),
        Err(error) => json!({"status": "failed", "error": format!("{error:#}")}),
    }
}

fn synchronization_masked(root: &Path) -> Result<()> {
    [
        "systemd-timesyncd.service",
        "systemd-time-wait-sync.service",
    ]
    .into_iter()
    .try_for_each(|unit| -> Result<()> {
        let state = process::command(
            root,
            &format!("sync-{unit}"),
            std::process::Command::new("/usr/bin/systemctl").args([
                "show",
                "--property=LoadState",
                "--property=ActiveState",
                unit,
            ]),
            100,
        )?;
        validate_synchronization_state(&state, unit)?;
        Ok(())
    })
}

fn validate_synchronization_state(state: &str, unit: &str) -> Result<()> {
    let mut lines = state.lines();
    let properties = [lines.next(), lines.next()];
    ensure!(
        lines.next().is_none(),
        "extra synchronization properties: {unit}"
    );
    let active = match properties {
        [Some("LoadState=masked"), Some(active)] | [Some(active), Some("LoadState=masked")] => {
            active
        }
        _ => anyhow::bail!("guest synchronization not masked: {unit}: {state}"),
    };
    ensure!(
        matches!(active, "ActiveState=inactive" | "ActiveState=failed"),
        "guest synchronization still active: {unit}: {state}"
    );
    Ok(())
}

fn validate_set_clock(before: &Value, after: &Value, date: &str) -> Result<()> {
    for field in ["boot_id", "machine_id"] {
        let previous = before
            .get(field)
            .and_then(Value::as_str)
            .with_context(|| format!("before {field} absent"))?;
        let current = after
            .get(field)
            .and_then(Value::as_str)
            .with_context(|| format!("after {field} absent"))?;
        ensure!(
            !previous.is_empty() && previous == current,
            "guest {field} changed or empty"
        );
    }
    let actual = chrono::DateTime::parse_from_rfc3339(
        after
            .get("realtime")
            .and_then(Value::as_str)
            .context("guest realtime absent")?,
    )?
    .with_timezone(&chrono::Utc);
    let day = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")?;
    ensure!(
        actual.date_naive() == day,
        "actual guest date differs from target"
    );
    ensure!(
        after.get("date").and_then(Value::as_str) == Some(date),
        "reported guest date differs from target"
    );
    let target = day
        .and_hms_opt(23, 55, 0)
        .context("guest clock target invalid")?;
    let upper = target
        .checked_add_signed(chrono::Duration::seconds(10))
        .context("guest clock target upper bound overflow")?;
    ensure!(
        target <= actual.naive_utc() && actual.naive_utc() < upper,
        "actual guest clock injection missed target"
    );
    Ok(())
}

pub(super) fn clock() -> Result<Value> {
    let now = artifacts::now();
    let date = now.get(..10).context("guest RFC3339 day absent")?;
    Ok(
        json!({"realtime":now,"date":date,"monotonic_uptime":std::fs::read_to_string("/proc/uptime")?.trim(),"boot_id":std::fs::read_to_string("/proc/sys/kernel/random/boot_id")?.trim(),"machine_id":std::fs::read_to_string("/etc/machine-id")?.trim()}),
    )
}
