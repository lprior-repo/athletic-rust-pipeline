use super::{checks, Record, URL};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};

pub(in super::super::super) fn verify(before: &Value, after: &Value) -> Result<Value> {
    let first = record(before)?;
    let second = record(after)?;
    checks::crossing(&first, &second)?;
    ensure!(
        first.paths.metadata_sha256 != second.paths.metadata_sha256
            && first.journal.key != second.journal.key,
        "new acquisitions reused the same immutable capture or journal identity"
    );
    Ok(json!({
        "before":before,"after":after,"same_identity":first.identity,
        "fresh_physical_responses":2,"distinct_capture_manifests":true,
        "production_timestamp_validation":true
    }))
}

fn record(value: &Value) -> Result<Record> {
    ensure!(
        value.get("store_flushed") == Some(&json!(true))
            && value.get("store_closed") == Some(&json!(true)),
        "acquisition was not durably finalized by its sole owner"
    );
    let record: Record = serde_json::from_value(
        value
            .get("acquisition")
            .context("physical acquisition absent")?
            .clone(),
    )?;
    ensure!(
        record.capture.url == URL
            && record.capture.method == "GET"
            && record.capture.status == 200
            && !record.capture.from_cache
            && record.capture.bytes > 0
            && record.stats.physical_requests() == 1
            && record.stats.cache_hits == 0
            && record.stats.conditional_304 == 0
            && record.stats.errors == 0
            && record.stats.bytes_downloaded == u64::try_from(record.capture.bytes)?
            && record.schools > 0
            && record.xc_tf_appointments > 0,
        "certificate is not a fresh successful full-body production acquisition"
    );
    checks::acquisition_time(&record.capture, &record.clock_before, &record.clock)?;
    Ok(record)
}
