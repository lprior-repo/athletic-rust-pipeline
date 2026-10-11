use crate::args::{Args, Fault};
use crate::report::write_json;
use crate::rows::{decode_row_key, group_rows, hex, row_index};
use crate::witness::walk;
use census_store::{NativeEffectCheckpoint, Receipt};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

const CHECKPOINT_ROW_CAP: usize = 100_000;

fn by_operation(checkpoint: &NativeEffectCheckpoint) -> BTreeMap<&str, &Receipt> {
    checkpoint
        .receipts
        .iter()
        .map(|receipt| (receipt.operation.as_str(), receipt))
        .collect()
}

pub fn receipts(
    marker: &NativeEffectCheckpoint,
    copy: &NativeEffectCheckpoint,
    log: &mut Vec<String>,
) -> Result<BTreeSet<String>, Fault> {
    let marker_receipts = by_operation(marker);
    let copy_receipts = by_operation(copy);
    for (operation, expected) in &marker_receipts {
        match copy_receipts.get(operation) {
            None => {
                return Err(Fault::verify(format!(
                    "acknowledged receipt {operation} lost (digest={} appended={})",
                    expected.digest, expected.appended
                )))
            }
            Some(found) => {
                if found.digest != expected.digest
                    || found.at != expected.at
                    || found.appended != expected.appended
                {
                    return Err(Fault::verify(format!(
                        "acknowledged receipt {operation} mutated (expected digest={} at={} appended={}; found digest={} at={} appended={})",
                        expected.digest, expected.at, expected.appended, found.digest, found.at, found.appended
                    )));
                }
                if found.credited && !expected.credited {
                    log.push(format!(
                        "INFO: receipt {operation} was credited after the boundary snapshot"
                    ));
                }
            }
        }
    }
    Ok(marker_receipts.keys().map(|key| key.to_string()).collect())
}

pub fn rows(marker: &NativeEffectCheckpoint, copy: &NativeEffectCheckpoint) -> Result<(), Fault> {
    let copy_rows = row_index(&copy.rows);
    for row in &marker.rows {
        let identity = (row.table.file().to_string(), hex(&row.key));
        match copy_rows.get(&identity) {
            None => {
                return Err(Fault::verify(format!(
                    "acknowledged row lost: table={} id={} digest={}",
                    row.table.file(),
                    decode_row_key(&row.table, &row.key),
                    row.digest
                )))
            }
            Some(digest) if digest != &row.digest => {
                return Err(Fault::verify(format!(
                    "acknowledged row mutated: table={} id={} expected digest={} found digest={}",
                    row.table.file(),
                    decode_row_key(&row.table, &row.key),
                    row.digest,
                    digest
                )))
            }
            Some(_) => {}
        }
    }
    Ok(())
}

pub fn duplicate_commits(
    marker: &NativeEffectCheckpoint,
    copy: &NativeEffectCheckpoint,
    args: &Args,
    marker_operations: &BTreeSet<String>,
) -> Result<(), Fault> {
    let marker_receipts = by_operation(marker);
    let marker_digests: BTreeSet<&str> = marker
        .receipts
        .iter()
        .map(|receipt| receipt.digest.as_str())
        .collect();
    let mut extra_commits: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for receipt in &copy.receipts {
        if !marker_operations.contains(receipt.operation.as_str())
            && marker_digests.contains(receipt.digest.as_str())
        {
            extra_commits
                .entry(receipt.digest.as_str())
                .or_default()
                .push(receipt.operation.as_str());
        }
    }
    if extra_commits.is_empty() {
        return Ok(());
    }
    write_json(
        &args.evidence.join("dup-receipts.json"),
        &json!(extra_commits),
    )?;
    let described = extra_commits
        .iter()
        .map(|(digest, operations)| {
            let pre = marker_receipts
                .values()
                .find(|receipt| receipt.digest.as_str() == *digest)
                .map_or("", |receipt| receipt.operation.as_str());
            format!(
                "content {digest} was committed a second time under new receipt keys {operations:?} (pre-boundary key {pre})"
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    Err(Fault::verify(format!(
        "duplicate receipt commit: {described}"
    )))
}

pub fn groups(
    marker: &NativeEffectCheckpoint,
    copy: &NativeEffectCheckpoint,
    args: &Args,
) -> Result<(), Fault> {
    let marker_row_counts = group_rows(&marker.rows);
    let copy_row_counts = group_rows(&copy.rows);
    for (group, count) in &marker_row_counts {
        if let Some(seen) = copy_row_counts.get(group) {
            if seen > count {
                write_json(
                    &args.evidence.join("dup-rows.json"),
                    &json!({
                        "table": group.0,
                        "id": group.1,
                        "digest": group.2,
                        "marker_count": count,
                        "copy_count": seen,
                    }),
                )?;
                return Err(Fault::verify(format!(
                    "acknowledged observation committed twice: table={} id={} digest={} marker count={} store count={}",
                    group.0, group.1, group.2, count, seen
                )));
            }
        }
    }
    Ok(())
}

pub fn pending(copy_dir: &std::path::Path) -> Result<(), Fault> {
    for path in walk(copy_dir)?.keys() {
        if path.ends_with(".pending") {
            return Err(Fault::verify(format!(
                "staged but unpublished batch inside the store copy: {path}"
            )));
        }
    }
    Ok(())
}

pub fn new_counts(
    marker: &NativeEffectCheckpoint,
    copy: &NativeEffectCheckpoint,
    marker_operations: &BTreeSet<String>,
    orphan_flags: &mut Vec<String>,
    limits: &mut Vec<String>,
) -> Result<(u64, usize), Fault> {
    let rows_new = u64::try_from(copy.rows.len().saturating_sub(marker.rows.len()))
        .map_err(|_| Fault::io("observation row count exceeds u64"))?;
    let receipts_new = copy.receipts.len().saturating_sub(marker.receipts.len());
    let appended_new: u64 = copy
        .receipts
        .iter()
        .filter(|receipt| !marker_operations.contains(receipt.operation.as_str()))
        .map(|receipt| receipt.appended)
        .sum();
    if rows_new > appended_new {
        orphan_flags.push(format!(
            "{rows_new} observation rows appeared across the copy while new receipts append only {appended_new}"
        ));
    }
    if copy.rows.len() >= CHECKPOINT_ROW_CAP || marker.rows.len() >= CHECKPOINT_ROW_CAP {
        limits.push(format!(
            "checkpoint row cap {CHECKPOINT_ROW_CAP} reached; very large stores are refused at checkpoint time instead of covered"
        ));
    }
    limits.push(
        "journal, meta, derived-generation, capture-cache and endpoint-credit effects are outside the checkpoint".to_string(),
    );
    Ok((rows_new, receipts_new))
}
