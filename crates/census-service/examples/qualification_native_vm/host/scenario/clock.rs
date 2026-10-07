use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::path::Path;
use std::time::{Duration, Instant};

use super::super::super::{artifacts, transport::Ssh};
use super::clock_window;

pub(super) fn midnight(ssh: &Ssh, root: &Path) -> Result<Value> {
    ssh.checkpoint()?;
    let injected = clock_window::inject(ssh, root)?;
    let deadline = Instant::now()
        .checked_add(Duration::from_secs(330))
        .context("natural midnight monotonic deadline overflow")?;
    artifacts::publish(
        &root.join("guest-clock-injection.json"),
        &json!({"guest":injected,"host_at":artifacts::now()}),
    )?;
    clock_window::validate_injection(&injected)?;
    let active = ssh.action("sweep-clock-start")?;
    let acquired_before = ssh.action("clock-acquire-before")?;
    artifacts::publish(
        &root.join("guest-fresh-acquisition-before-midnight.json"),
        &acquired_before,
    )?;
    let before = ssh.clock_until(deadline)?;
    artifacts::publish(
        &root.join("guest-clock-sleep-baseline.json"),
        &json!({"injection":injected,"unfinished_sleep_witness":active,"baseline":before,"host_at":artifacts::now()}),
    )?;
    let baseline_proof = validate_clock_baseline(&injected, &active, &before)?;
    let after = cross_midnight(ssh, &before, deadline)?;
    let acquired_after = ssh.action("clock-acquire-after")?;
    artifacts::publish(
        &root.join("guest-fresh-acquisition-after-midnight.json"),
        &acquired_after,
    )?;
    ensure!(
        acquired_before.pointer("/acquisition/identity").is_some()
            && acquired_before.pointer("/acquisition/identity")
                == acquired_after.pointer("/acquisition/identity"),
        "fresh acquisition run/source/cohort/season/revision identity changed at midnight"
    );
    let replay = ssh.action("repeat-clock")?;
    ensure!(
        replay
            .get("reply")
            .and_then(|value| value.get("last_appended_at"))
            == after.get("date"),
        "new Ingest execution timestamp dishonest after midnight"
    );
    let workflow = ssh.action("sweep-clock-finish")?;
    artifacts::publish(
        &root.join("clock-oracle.json"),
        &json!({"verdict":"PASS","injection":injected,"before":before,"after":after,"active_before":active,"baseline_proof":baseline_proof,"same_sweep_journal_date":workflow,"same_effect_replay":replay,"fresh_before":acquired_before,"fresh_after":acquired_after,"host_after":artifacts::now(),"limit":"isolated guest acquisition clock proof; sampled realtime and uptime must not regress and elapsed discrepancy must be at most one second both from the previous validated sample and the original baseline, with no rate allowance; two-decimal uptime quantization and sequential measurement are included in this total allowance, not proof against unsampled, cancelling or sub-tolerance clock adjustments; not host-clock freshness or national census acceptance"}),
    )?;
    offline_snapshot(ssh, root, "after-clock")
}

pub(super) fn offline_snapshot(ssh: &Ssh, root: &Path, label: &str) -> Result<Value> {
    ssh.exec(
        "offline-stop",
        "/usr/bin/systemctl stop qualification.service",
    )?;
    let output = ssh.exec("offline-snapshot", "/srv/qualification/lib/ld-linux-x86-64.so.2 --library-path /srv/qualification/lib /srv/qualification/qualification guest-action --action snapshot")?;
    let value: Value = serde_json::from_str(output.trim())?;
    artifacts::publish(
        &root.join(format!("{label}-physical-snapshot.json")),
        &value,
    )?;
    Ok(value)
}

pub(super) fn cross_midnight(ssh: &Ssh, before: &Value, deadline: Instant) -> Result<Value> {
    let mut progress = clock_window::ClockProgress::new(before)?;
    let checks = (0..330).find_map(|_| -> Option<Result<Value>> {
        let observed = (|| -> Result<Option<Value>> {
            ssh.checkpoint()?;
            ensure!(
                Instant::now() < deadline,
                "natural guest midnight crossing exceeded 330-second injection budget"
            );
            let value = ssh.clock_until(deadline)?;
            ensure!(
                Instant::now() < deadline,
                "guest clock observation returned after the 330-second crossing budget"
            );
            if progress.observe(&value)? {
                return Ok(Some(value));
            }
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .context("natural midnight deadline expired before polling pause")?;
            ssh.pause(remaining.min(Duration::from_secs(1)))?;
            Ok(None)
        })();
        match observed {
            Ok(Some(value)) => Some(Ok(value)),
            Ok(None) => None,
            Err(error) => Some(Err(error)),
        }
    });
    checks.context("actual guest UTC date did not cross midnight within fixed budget")?
}

pub(super) fn validate_clock_baseline(
    injected: &Value,
    active: &Value,
    baseline: &Value,
) -> Result<Value> {
    let injection = injected.get("after").context("injected clock absent")?;
    let original = active
        .get("invocation")
        .context("original invocation absent")?;
    let workflow = original
        .get("starting_clock")
        .context("unfinished Sleep workflow starting clock absent")?;
    clock_window::validate_machine(injection, workflow)?;
    clock_window::validate_baseline(injected, baseline)?;
    let day = clock_field(injection, "date")?;
    let boot = clock_field(injection, "boot_id")?;
    ensure!(
        clock_field(workflow, "date")? == day && clock_field(baseline, "date")? == day,
        "missed pre-midnight injection window: Sleep witness baseline differs from injected date"
    );
    ensure!(
        clock_field(workflow, "boot_id")? == boot && clock_field(baseline, "boot_id")? == boot,
        "clock setup or unfinished Sleep baseline changed guest boot identity"
    );
    let boundary = active
        .get("boundary")
        .context("unfinished Sleep journal absent")?;
    let observation = boundary
        .get("observation")
        .context("unfinished Sleep observation absent")?;
    let id = clock_field(original, "id")?;
    let key = clock_field(original, "key")?;
    let accepted = original
        .get("accepted")
        .context("original acknowledgement absent")?;
    ensure!(
        !id.is_empty()
            && key == super::super::super::CLOCK_WORKFLOW
            && clock_field(accepted, "invocationId")? == id
            && clock_field(observation, "id")? == id
            && clock_field(observation, "key")? == key,
        "clock Sleep does not belong to the acknowledged original clock workflow"
    );
    super::super::super::native::sleep::unfinished(observation, baseline)?
        .context("pre-midnight baseline lacks actual unfinished Sleep witness")
}

#[cfg(test)]
pub(super) fn crossed(before: &Value, after: &Value) -> Result<bool> {
    clock_window::validate_machine(before, after)?;
    let before_day = chrono::NaiveDate::parse_from_str(clock_field(before, "date")?, "%Y-%m-%d")?;
    let after_day = chrono::NaiveDate::parse_from_str(clock_field(after, "date")?, "%Y-%m-%d")?;
    clock_window::crossed_day(before_day, after_day)
}

pub(super) fn clock_field<'a>(clock: &'a Value, name: &str) -> Result<&'a str> {
    clock
        .get(name)
        .and_then(Value::as_str)
        .with_context(|| format!("guest clock {name} absent"))
}
