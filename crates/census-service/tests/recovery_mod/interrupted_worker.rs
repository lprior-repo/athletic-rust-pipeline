#![cfg(feature = "native-fault-injection")]

use std::collections::BTreeSet;

use census_domain::model::{CanonicalMeet, SourceNamespace};
use serde_json::{json, Value};

use super::process::{report_of, run_census};
use super::{meet_ids, open_store, Table};

#[path = "native_worker.rs"]
mod native_worker;
use native_worker::{certificate, kill_at_boundary, receipts, SourceInput, DERIVED};

#[test]
fn sigkill_mid_batch_worker_restart_completes_the_remaining_units() -> super::TestResult {
    let directory = tempfile::tempdir()?;
    let input = SourceInput::write(directory.path(), 4096)?;
    let args = input.args()?;
    let control_root = directory.path().join("control");
    run_census(&control_root, &args)?;
    let control = open_store(&control_root)?;
    let expected_ids = meet_ids(&control)?;
    let expected_receipts = receipts(&control)?;
    let expected_checkpoint = control.native_effect_checkpoint()?;
    let native: BTreeSet<_> = control.scan::<CanonicalMeet>(Table::Meets)?.into_iter()
        .flat_map(|meet| meet.source_identities.into_iter())
        .filter(|identity| matches!(&identity.namespace, SourceNamespace::TimerMeet { provider } if provider == "athleticlive"))
        .map(|identity| identity.id).collect();
    let published: BTreeSet<_> = (50_001..54_097).map(|id| id.to_string()).collect();
    check!(eq; native, published);
    check!(eq; expected_ids.len(), 4096);
    check!(eq; expected_receipts.len(), 4096);
    for phase in [
        "source_batch_staged_before_commit",
        "source_chunk_committed_before_next",
    ] {
        let root = directory.path().join(phase);
        let marker = kill_at_boundary(&root, &args, phase)?;
        let operation = marker
            .get("operation")
            .and_then(Value::as_str)
            .ok_or("missing operation")?;
        let digest = marker
            .get("input_digest")
            .and_then(Value::as_str)
            .ok_or("missing digest")?;
        let durable = open_store(&root)?;
        let before_ids = meet_ids(&durable)?;
        let before_receipts = receipts(&durable)?;
        let before = durable.native_effect_checkpoint()?;
        let committed = phase == "source_chunk_committed_before_next";
        check!(eq; before_ids.len(), usize::from(committed));
        check!(eq; before_receipts.len(), usize::from(committed));
        check!(eq; before.rows.len(), usize::from(committed));
        check!(eq; durable.receipt(operation)?.is_some(), committed);
        if let Some(receipt) = durable.receipt(operation)? {
            check!(eq; receipt.digest, digest);
            check!(eq; Some(&receipt), expected_receipts.get(operation));
        }
        check!(before_ids.is_subset(&expected_ids));
        check!(before
            .rows
            .iter()
            .all(|row| expected_checkpoint.rows.contains(row)));
        let remaining: BTreeSet<_> = expected_ids.difference(&before_ids).cloned().collect();
        check!(!remaining.is_empty());
        drop(durable);
        let recovered = report_of(&run_census(&root, &args)?)?;
        check!(eq; recovered.get("rows").and_then(Value::as_u64), Some(u64::try_from(remaining.len())?));
        let store = open_store(&root)?;
        let recovered_ids = meet_ids(&store)?;
        let recovered_receipts = receipts(&store)?;
        check!(eq; &recovered_ids, &expected_ids);
        check!(eq; &recovered_receipts, &expected_receipts);
        check!(eq; store.native_effect_checkpoint()?.rows, expected_checkpoint.rows);
        let receipt = store
            .receipt(operation)?
            .ok_or("original staged operation was not recovered")?;
        check!(eq; receipt.digest, digest);
        let stable = store.native_effect_checkpoint()?;
        drop(store);
        check!(eq; report_of(&run_census(&root, &args)?)?.get("rows").and_then(Value::as_u64), Some(0));
        let reopened = open_store(&root)?;
        check!(eq; reopened.native_effect_checkpoint()?.rows, stable.rows);
        check!(eq; receipts(&reopened)?, recovered_receipts);
        certificate(
            phase,
            json!({
                "schema":1,"phase":phase,"marker":marker,"killed_pid":marker["pid"],"signal":9,
                "same_operation_recovered":true,"atomic_visibility_proven":true,"exact_remaining_recovery_proven":true,
                "expected_ids":expected_ids,"recovered_ids":recovered_ids,
                "expected_receipts":expected_receipts.keys().collect::<Vec<_>>(),
                "recovered_receipts":recovered_receipts.keys().collect::<Vec<_>>(),
                "remaining_ids_at_kill":remaining
            }),
        )?;
    }
    Ok(())
}

#[test]
fn sigkill_derived_generation_restart_never_exposes_partial_state() -> super::TestResult {
    const PHASE: &str = "derived_batch_staged_before_publish";
    let directory = tempfile::tempdir()?;
    let input = SourceInput::write(directory.path(), 2)?;
    let root = directory.path().join("killed");
    let control_root = directory.path().join("control");
    let index = ["index", "--school-year", "2026"];
    for store in [&root, &control_root] {
        run_census(store, &input.args()?)?;
        run_census(store, &index)?;
    }
    let before = open_store(&root)?;
    let previous_generation = before.derived_generation();
    let previous_digest = before.snapshot().tables_digest(&DERIVED)?;
    check!(previous_generation > 0);
    drop(before);
    let expanded = SourceInput::write(directory.path(), 3)?;
    for store in [&root, &control_root] {
        run_census(store, &expanded.args()?)?;
    }
    run_census(&control_root, &index)?;
    let control = open_store(&control_root)?;
    let expected_ids = meet_ids(&control)?;
    let expected_receipts = receipts(&control)?;
    let expected_digest = control.snapshot().tables_digest(&DERIVED)?;
    let source_checkpoint = control.native_effect_checkpoint()?;
    check!(
        expected_digest != previous_digest,
        "expanded source must change actual derived indexes"
    );
    let marker = kill_at_boundary(&root, &index, PHASE)?;
    let staged_generation = marker
        .get("generation")
        .and_then(Value::as_u64)
        .ok_or("missing staged generation")?;
    check!(staged_generation > previous_generation);
    let operation = marker
        .get("operation")
        .and_then(Value::as_str)
        .ok_or("missing index operation")?;
    let digest = marker
        .get("input_digest")
        .and_then(Value::as_str)
        .ok_or("missing index digest")?;
    let killed = open_store(&root)?;
    let visible_generation_after_kill = killed.derived_generation();
    check!(eq; visible_generation_after_kill, previous_generation);
    check!(eq; killed.snapshot().tables_digest(&DERIVED)?, previous_digest);
    check!(
        killed.receipt(operation)?.is_none(),
        "hidden staged batch is not an acknowledged publication"
    );
    check!(eq; killed.native_effect_checkpoint()?.rows, source_checkpoint.rows);
    drop(killed);
    run_census(&root, &index)?;
    let recovered = open_store(&root)?;
    check!(recovered.derived_generation() > staged_generation);
    check!(eq; recovered.snapshot().tables_digest(&DERIVED)?, expected_digest);
    let recovered_ids = meet_ids(&recovered)?;
    let recovered_receipts = receipts(&recovered)?;
    check!(eq; &recovered_ids, &expected_ids);
    check!(eq; &recovered_receipts, &expected_receipts);
    check!(eq; recovered.native_effect_checkpoint()?.rows, source_checkpoint.rows);
    check!(eq; recovered.receipt(operation)?.ok_or("original index operation not recovered")?.digest, digest);
    let complete_generation = recovered.derived_generation();
    drop(recovered);
    run_census(&root, &index)?;
    let replay = open_store(&root)?;
    check!(eq; replay.derived_generation(), complete_generation);
    check!(eq; replay.snapshot().tables_digest(&DERIVED)?, expected_digest);
    check!(eq; receipts(&replay)?, recovered_receipts);
    certificate(
        PHASE,
        json!({
            "schema":1,"phase":PHASE,"marker":marker,"killed_pid":marker["pid"],"signal":9,
            "same_operation_recovered":true,"atomic_visibility_proven":true,"exact_remaining_recovery_proven":true,
            "expected_ids":expected_ids,"recovered_ids":recovered_ids,
            "expected_receipts":expected_receipts.keys().collect::<Vec<_>>(),
            "recovered_receipts":recovered_receipts.keys().collect::<Vec<_>>(),
            "previous_generation":previous_generation,"staged_generation":staged_generation,
            "visible_generation_after_kill":visible_generation_after_kill,
            "staged_generation_not_visible":true,"complete_generation_published":true
        }),
    )?;
    Ok(())
}
