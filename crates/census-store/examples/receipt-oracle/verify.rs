use census_store::{Store, StoreError, DEFAULT_CACHE_BYTES};
use sha2::{Digest, Sha256};
use std::fs;

use crate::args::{Args, Fault, Marker};
use crate::checks;
use crate::report::{self, Verdict};
use crate::witness::{walk, witness_diff, witness_json};

const BASENAME: &str = "teams-source-reservation-reached.json";
const PENDING: &str = "teams-source-reservation-reached.json.pending";

pub fn verify(args: &Args, log: &mut Vec<String>) -> Result<Verdict, Fault> {
    let mut limits = Vec::new();
    let mut orphan_flags = Vec::new();

    log.push(format!(
        "INFO: receipt oracle for kind={} operation={} invocation={}",
        args.kind, args.operation, args.invocation
    ));
    let marker_bytes =
        fs::read(&args.marker).map_err(|error| Fault::io(format!("reading marker: {error}")))?;
    let marker_sha = format!("{:x}", Sha256::digest(&marker_bytes));
    let marker: Marker = serde_json::from_slice(&marker_bytes)
        .map_err(|error| Fault::io(format!("marker schema drift: {error}")))?;
    let marker_prefix = marker_sha
        .get(..16)
        .map_or(marker_sha.as_str(), std::convert::identity);
    log.push(format!(
        "INFO: marker schema={} phase={} attempt={} sha256={}",
        marker.schema, marker.phase, marker.attempt, marker_prefix
    ));

    if marker.schema != 2 {
        return Err(Fault::verify(format!(
            "marker is schema {} without acknowledged effects: the seam did not snapshot the acknowledged store state at this boundary",
            marker.schema
        )));
    }
    let Some(marker_checkpoint) = marker.acknowledged_effects.clone() else {
        return Err(Fault::verify(format!(
            "marker is schema {} without acknowledged effects: the seam did not snapshot the acknowledged store state at this boundary",
            marker.schema
        )));
    };
    if marker.phase != args.kind {
        return Err(Fault::verify(format!(
            "phase mismatch: marker has phase {} but expected {}",
            marker.phase, args.kind
        )));
    }
    if marker.operation != args.operation {
        return Err(Fault::verify(format!(
            "operation mismatch: marker has {} but expected {}",
            marker.operation, args.operation
        )));
    }
    if !(1..=3).contains(&marker.attempt) {
        return Err(Fault::verify(format!(
            "invalid attempt {} (expected 1..=3)",
            marker.attempt
        )));
    }
    if marker.request_digest.is_empty()
        || marker.request_digest.len() > 128
        || marker.request_digest == "fault_injection"
    {
        return Err(Fault::verify(format!(
            "request_digest out of bounds or placeholder: {:?}",
            marker.request_digest
        )));
    }
    if marker.observed_on.is_empty()
        || marker.observed_on.len() > 64
        || marker.observed_on == "fault_injection"
    {
        return Err(Fault::verify(format!(
            "observed_on out of bounds or placeholder: {:?}",
            marker.observed_on
        )));
    }
    log.push(format!(
        "INFO: identity gate passed (request_digest={}, observed_on={})",
        marker.request_digest, marker.observed_on
    ));
    if marker_checkpoint.schema != 1 {
        log.push(format!(
            "WARN: marker checkpoint schema {} is not 1",
            marker_checkpoint.schema
        ));
    }
    limits.push(
        "unbound invocation: neither the marker nor the store carries an invocation id".to_string(),
    );

    let boundary = args
        .store
        .parent()
        .ok_or_else(|| Fault::io("--store has no parent directory"))?
        .join("boundary");
    let original = boundary.join(BASENAME);
    if original.is_file() {
        let original_bytes = fs::read(&original)
            .map_err(|error| Fault::io(format!("reading original marker: {error}")))?;
        let original_sha = format!("{:x}", Sha256::digest(&original_bytes));
        if original_sha != marker_sha {
            return Err(Fault::verify(format!(
                "--marker is not the boundary file the endpoint published for this store (sha256 {marker_sha} vs {original_sha})"
            )));
        }
        log.push("INFO: marker provenance matches the published boundary file".to_string());
    } else {
        limits.push(format!(
            "original boundary marker {} not retained; provenance limited to content checks",
            original.display()
        ));
    }
    if boundary.join(PENDING).exists() {
        return Err(Fault::verify(format!(
            "orphaned marker publication: {} exists",
            boundary.join(PENDING).display()
        )));
    }

    let store_before = walk(&args.store)?;
    report::write_json(
        &args.evidence.join("store-before.json"),
        &witness_json(&store_before),
    )?;
    log.push(format!(
        "INFO: witness baseline recorded for {} store files",
        store_before.len()
    ));

    let copy_dir = args.evidence.join("store-copy");
    match Store::backup(&args.store, &copy_dir) {
        Ok(_) => log.push(format!("INFO: cold backup written to {}", copy_dir.display())),
        Err(StoreError::Refused { detail }) => {
            return Err(Fault::io(format!(
                "store is still owned by a live process; stop the endpoint before running the oracle ({detail})"
            )))
        }
        Err(error) => return Err(Fault::io(format!("cold backup failed: {error}"))),
    }

    let copy =
        Store::open(&copy_dir).map_err(|error| Fault::io(format!("opening copy: {error}")))?;
    let sequence_copy = copy.snapshot().sequence();
    let checkpoint_copy = copy
        .native_effect_checkpoint()
        .map_err(|error| Fault::io(format!("checkpointing copy: {error}")))?;
    let integrity = copy
        .integrity()
        .map_err(|error| Fault::io(format!("reading integrity: {error}")))?;
    report::write_json(
        &args.evidence.join("integrity.json"),
        &serde_json::to_value(&integrity).map_err(|error| Fault::io(error.to_string()))?,
    )?;
    report::write_json(
        &args.evidence.join("checkpoint-copy.json"),
        &serde_json::to_value(&checkpoint_copy).map_err(|error| Fault::io(error.to_string()))?,
    )?;
    match copy.http_cache_integrity() {
        Ok(findings) => log.push(format!(
            "INFO: http cache integrity reported {} findings (outside the checkpoint)",
            findings.len()
        )),
        Err(error) => limits.push(format!("http cache integrity unavailable: {error}")),
    }
    drop(copy);

    let store_after = walk(&args.store)?;
    report::write_json(
        &args.evidence.join("store-after.json"),
        &witness_json(&store_after),
    )?;
    let changed = witness_diff(&store_before, &store_after);
    if !changed.is_empty() {
        return Err(Fault::verify(format!(
            "the oracle modified the witness store at {:?}",
            changed
        )));
    }
    log.push("INFO: witness store unchanged by readback".to_string());

    match Store::inspect_with(&copy_dir, DEFAULT_CACHE_BYTES) {
        Ok(inspection) => {
            let published = inspection.derived_generation();
            match inspection.next_staging_generation() {
                Ok(next) if next > published => orphan_flags.push(format!(
                    "derived staging generation {next} exceeds the published generation {published}"
                )),
                Ok(_) => {}
                Err(error) => {
                    limits.push(format!("derived staging generation unreadable: {error}"))
                }
            }
        }
        Err(error) => limits.push(format!("store inspection unavailable: {error}")),
    }

    let sequence_marker = marker_checkpoint.sequence;
    if checkpoint_copy.sequence < sequence_marker {
        return Err(Fault::verify(format!(
            "marker snapshot sequence {sequence_marker} exceeds store sequence {}: the marker does not belong to this store history",
            checkpoint_copy.sequence
        )));
    }

    let marker_operations = checks::receipts(&marker_checkpoint, &checkpoint_copy, log)?;
    checks::rows(&marker_checkpoint, &checkpoint_copy)?;
    checks::duplicate_commits(
        &marker_checkpoint,
        &checkpoint_copy,
        args,
        &marker_operations,
    )?;
    checks::groups(&marker_checkpoint, &checkpoint_copy, args)?;

    if !integrity.ok {
        return Err(Fault::verify("store integrity is not clean".to_string()));
    }
    for table in &integrity.tables {
        if !table.details.is_empty() {
            return Err(Fault::verify(format!(
                "integrity detail for table {}: {}",
                table.table,
                table.details.join("; ")
            )));
        }
    }
    if !integrity.unreadable_journals.is_empty() || !integrity.unreadable_entity_logs.is_empty() {
        return Err(Fault::verify(format!(
            "integrity found unreadable journals {:?} and entity logs {:?}",
            integrity.unreadable_journals, integrity.unreadable_entity_logs
        )));
    }

    checks::pending(&copy_dir)?;
    let (rows_new, receipts_new) = checks::new_counts(
        &marker_checkpoint,
        &checkpoint_copy,
        &marker_operations,
        &mut orphan_flags,
        &mut limits,
    )?;

    for flag in &orphan_flags {
        log.push(format!("FLAG: {flag}"));
    }
    for limit in &limits {
        log.push(format!("LIMIT: {limit}"));
    }
    let verdict = Verdict {
        sequence_marker,
        sequence_copy,
        rows_marker: marker_checkpoint.rows.len(),
        rows_copy: checkpoint_copy.rows.len(),
        receipts_marker: marker_checkpoint.receipts.len(),
        receipts_copy: checkpoint_copy.receipts.len(),
        rows_new,
        receipts_new,
        integrity_ok: integrity.ok,
        copy_dir,
        limits,
        orphan_flags,
        marker_sha,
    };
    report::publish(args, &verdict, log)?;
    Ok(verdict)
}
