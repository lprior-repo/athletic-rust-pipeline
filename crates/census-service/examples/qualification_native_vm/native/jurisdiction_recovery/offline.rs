use super::super::super::{artifacts, oracle, GUEST};
use super::{captures, input};
use anyhow::{ensure, Context, Result};
use census_store::Store;
use serde_json::{json, Value};
use std::path::Path;

pub(super) fn read() -> Result<Value> {
    let root = Path::new(GUEST);
    let drain = oracle::snapshot::drain_certificate()?;
    let original = input::load()?;
    let finished = artifacts::json(&root.join("jurisdiction-recovery-finished.json"))?;
    ensure!(
        finished
            .get("reattached_invocation_id")
            .and_then(Value::as_str)
            == Some(&original.id)
            && finished.get("source_stage_recovery_observed") == Some(&json!(true)),
        "offline source oracle has no recovered original invocation"
    );
    super::recovery::verify_finished(&finished, &original)?;
    let store = Store::open(root.join("store"))?;
    let ledgers = source_ledgers(&store, &finished)?;
    let reservation = reservation(&store)?;
    let acknowledged_effects = super::effects::read(&store)?;
    let retained = retained_captures(&original, &finished)?;
    let snapshot = store.snapshot();
    let corpus = oracle::corpus::read(&snapshot)?;
    Ok(json!({
        "original_invocation_id":original.id,"source_ledgers":ledgers,
        "original_registration":reservation.get("registration"),
        "original_reservation":reservation.get("reservation"),
        "captures":retained,"corpus":corpus,"drain_certificate":drain,
        "acknowledged_effects":acknowledged_effects,
        "parent_recovery":finished.get("parent_recovery").context("parent recovery disposition absent")?,
        "single_owner_offline":true,
        "scope":"cold exact source-stage journal and physical corpus readback; no HTTP subphase claim"
    }))
}

fn source_ledgers(store: &Store, finished: &Value) -> Result<Vec<Value>> {
    let sources = finished
        .pointer("/reconciliation/sources")
        .and_then(Value::as_array)
        .context("recovered sources absent")?;
    ensure!(
        !sources.is_empty() && sources.len() <= 32,
        "source ledger budget invalid"
    );
    sources.iter().map(|source| ledger(store, source)).collect()
}

fn reservation(store: &Store) -> Result<Value> {
    let before = artifacts::json(&Path::new(GUEST).join("jurisdiction-recovery-boundary.json"))?;
    let witness = before
        .pointer("/boundary/witness")
        .context("original source witness absent")?;
    let key = witness
        .pointer("/source_call/child_key")
        .and_then(Value::as_str)
        .context("original child key absent")?;
    let registered = witness
        .pointer("/native_source_boundary/registration")
        .context("journaled original registration absent")?;
    ensure!(
        store
            .journal_payload("teams_source_attempts_v1", &format!("{key}/identity"))?
            .as_ref()
            == Some(registered),
        "original journaled source registration differs from cold Fjall readback"
    );
    let marker = witness
        .pointer("/native_source_boundary/marker/content")
        .context("original reservation marker content absent")?;
    let reservation = store
        .journal_payload(
            "teams_source_attempts_v1",
            &format!("{key}/attempt/1/reserved"),
        )?
        .context("acknowledged original source reservation absent after reboot")?;
    ensure!(
        reservation.get("attempt") == marker.get("attempt")
            && reservation.get("observed_on") == marker.get("observed_on"),
        "acknowledged source reservation changed across reboot"
    );
    Ok(json!({"registration":registered,"reservation":reservation}))
}

fn retained_captures(
    original: &input::Original,
    finished: &Value,
) -> Result<Vec<captures::CaptureRef>> {
    let retained = captures::read(&original.key)?;
    let recovered: Vec<captures::CaptureRef> = serde_json::from_value(
        finished
            .pointer("/reconciliation/captures")
            .context("recovered capture identities absent")?
            .clone(),
    )?;
    captures::reconcile(&recovered, &retained)?;
    Ok(retained)
}

fn ledger(store: &Store, source: &Value) -> Result<Value> {
    let key = input::text(source, "source_unit")?;
    let inspection = source
        .get("inspection")
        .context("source inspection absent")?;
    let references = source
        .get("source_store_journal_references")
        .and_then(Value::as_array)
        .context("source journal references absent")?;
    ensure!(
        !references.is_empty() && references.len() <= 7,
        "source journal reference budget invalid"
    );
    let values = references
        .iter()
        .map(|reference| journal(store, key, reference))
        .collect::<Result<Vec<_>>>()?;
    let settled = store
        .journal_payload("teams_source_attempts_v1", &format!("{key}/settled"))?
        .context("original source has no durable settled outcome")?;
    ensure!(
        inspection.get("outcome") == Some(&settled),
        "physical settled source differs from original inspection"
    );
    Ok(
        json!({"source_unit":key,"original_child_id":source.get("original_child_id"),
        "entries":values,"settled":settled}),
    )
}

fn journal(store: &Store, key: &str, reference: &Value) -> Result<Value> {
    let phase = input::text(reference, "phase")?;
    let slot = input::text(reference, "key")?;
    ensure!(
        phase == "teams_source_attempts_v1" && slot.starts_with(&format!("{key}/")),
        "source journal reference escapes original unit"
    );
    let payload = store
        .journal_payload(phase, slot)?
        .context("referenced physical source journal missing")?;
    Ok(json!({"reference":reference,"payload":payload}))
}
