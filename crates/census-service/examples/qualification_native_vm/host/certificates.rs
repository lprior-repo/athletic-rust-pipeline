use super::super::{artifacts, native};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::path::Path;

pub(super) fn verify(root: &Path) -> Result<Value> {
    let reboot = reboot(root)?;
    let clock = clock(root)?;
    let cleanup = artifacts::json(&root.join("cleanup.json"))?;
    ensure!(
        cleanup.get("qemu_orderly") == Some(&json!(true))
            && cleanup.get("qemu_state").and_then(Value::as_str) == Some("ORDERLY")
            && cleanup.get("disks_and_logs_preserved") == Some(&json!(true))
            && cleanup.get("failure") == Some(&Value::Null)
            && cleanup
                .get("guest_drain_certificate")
                .is_some_and(Value::is_object),
        "successful qualification requires owned VM reap and verified guest drain"
    );
    let physical = artifacts::json(&root.join("measured-durability-oracles.json"))?;
    ensure!(
        physical.get("verdict").and_then(Value::as_str) == Some("PASS"),
        "exact acknowledged effect/capture reconciliation absent"
    );
    Ok(
        json!({"scenario03":reboot,"scenario12":clock,"cleanup":cleanup,
        "acknowledged_effects":physical,"national_census_claim":false,
        "scope":"catalog03 reserved active source-stage reboot and catalog12 fresh guest acquisition midnight; HTTP fault subphases and scenario17 are separately required"}),
    )
}

fn reboot(root: &Path) -> Result<Value> {
    let source = artifacts::json(&root.join("host-source-active-before-reset.json"))?;
    let witness = source
        .pointer("/boundary/witness")
        .context("active source witness absent")?;
    ensure!(
        witness.get("boundary").and_then(Value::as_str)
            == Some("active_unfinished_production_teams_source"),
        "real active source boundary absent"
    );
    ensure!(
        witness
            .pointer("/native_source_boundary/proof")
            .and_then(Value::as_str)
            == Some("reserved_before_acquisition"),
        "published production reservation certificate absent"
    );
    let observed = artifacts::json(&root.join("host-reset-boundary-order.json"))?;
    let reset = artifacts::json(&root.join("qmp-reset.json"))?;
    let boundary_at =
        chrono::DateTime::parse_from_rfc3339(text(&observed, "host_boundary_received_at")?)?;
    let reset_at = chrono::DateTime::parse_from_rfc3339(text(&reset, "host_sent_at")?)?;
    ensure!(
        boundary_at <= reset_at && observed.get("qmp_reset") == Some(&reset),
        "reset is not ordered after the reached original boundary"
    );
    let recovery = artifacts::json(&root.join("host-source-recovered-after-reset.json"))?;
    ensure!(
        source.pointer("/original/id") == recovery.get("reattached_invocation_id")
            && recovery.pointer("/reconciliation/same_original_recovered") == Some(&json!(true))
            && recovery.pointer("/reconciliation/replacement_submitted") == Some(&json!(false)),
        "original active JurisdictionCensus did not recover"
    );
    let original_clock = source
        .pointer("/original/clock")
        .context("original guest clock absent")?;
    let recovered_clock = recovery
        .pointer("/reconciliation/clock")
        .context("recovered guest clock absent")?;
    ensure!(
        text(original_clock, "boot_id")? != text(recovered_clock, "boot_id")?
            && text(original_clock, "machine_id")? == text(recovered_clock, "machine_id")?,
        "actual guest reboot or retained machine identity is unproven"
    );
    let cold = artifacts::json(&root.join("after-reboot-physical-snapshot.json"))?;
    let ledger = cold
        .get("source_recovery")
        .context("cold original source ledger absent")?;
    ensure!(
        ledger.get("original_invocation_id") == source.pointer("/original/id")
            && ledger.get("single_owner_offline") == Some(&json!(true)),
        "offline source effect readback does not bind the original invocation"
    );
    ensure!(
        ledger.pointer("/acknowledged_effects/preserved") == Some(&json!(true))
            && ledger.pointer("/acknowledged_effects/before")
                == source.pointer(
                    "/boundary/witness/native_source_boundary/marker/content/acknowledged_effects"
                ),
        "pre-reset acknowledged raw source occurrences or receipt conservation is unproven"
    );
    let quiescent_before = artifacts::json(&root.join("source-quiescent-before-midnight.json"))?;
    let quiescent_after = artifacts::json(&root.join("source-quiescent-after-midnight.json"))?;
    ensure!(
        quiescent_before == quiescent_after
            && quiescent_before.get("original_invocation_id") == source.pointer("/original/id")
            && quiescent_before.get("quiescent") == Some(&json!(true))
            && quiescent_before.get("replacement_submitted") == Some(&json!(false)),
        "original parent and descendant quiescence across midnight is unproven"
    );
    Ok(
        json!({"reached_boundary":source,"reset":observed,"recovered":recovery,
        "cold_source_effects":ledger,"quiescence_before":quiescent_before,"quiescence_after":quiescent_after,"physical_http_subphase_claim":false}),
    )
}

fn clock(root: &Path) -> Result<Value> {
    let before = artifacts::json(&root.join("guest-fresh-acquisition-before-midnight.json"))?;
    let after = artifacts::json(&root.join("guest-fresh-acquisition-after-midnight.json"))?;
    let acquired = native::clock_acquisition::certificate::verify(&before, &after)?;
    let crossing = artifacts::json(&root.join("clock-oracle.json"))?;
    ensure!(
        crossing.get("verdict").and_then(Value::as_str) == Some("PASS")
            && crossing.get("fresh_before") == Some(&before)
            && crossing.get("fresh_after") == Some(&after),
        "midnight certificate differs from physical acquisitions"
    );
    Ok(
        json!({"physical_acquisitions":acquired,"natural_crossing":crossing,
        "host_clock_modified":false,"public_freshness_claim":false}),
    )
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .with_context(|| format!("required {key} absent"))
}
