use super::super::capture;
use super::super::projection::Input;
use super::{budget, receipt, Footprint, Window};
use crate::net::FetchOutcome;
use crate::{AdapterContext, CrawlError, CrawlResult};
use sha2::Digest;

pub(super) fn window(
    entry: &mut (String, serde_json::Value),
    input: &Input<'_>,
    window: &Window,
) -> CrawlResult<()> {
    let fields = entry.1.as_object_mut().ok_or_else(invalid)?;
    fields.insert("disposition".into(), serde_json::json!("window_applied"));
    fields.insert("unfinished".into(), serde_json::json!(true));
    fields.insert("window_index".into(), serde_json::json!(window.ordinal));
    fields.insert(
        "first_owned_index".into(),
        serde_json::json!(window.indices.first()),
    );
    fields.insert(
        "last_owned_index".into(),
        serde_json::json!(window.indices.last()),
    );
    fields.insert(
        "admitted_bytes".into(),
        serde_json::json!(window.footprint.bytes),
    );
    fields.insert(
        "admitted_work".into(),
        serde_json::json!(window.footprint.work),
    );
    entry.0 = receipt::identified(input.reference, &entry.1)?;
    Ok(())
}

pub(super) fn rejected(
    input: &Input<'_>,
    metadata: &FetchOutcome,
    index: usize,
    footprint: Footprint,
) -> CrawlResult<(String, serde_json::Value)> {
    let owned = input
        .acquired
        .result_set(&input.reference.rsid, input.performance_as_of)
        .ok_or_else(invalid)?;
    let row = owned.page.rows.get(index).ok_or_else(invalid)?;
    let payload = serde_json::json!({
        "meet": input.reference.meet_id, "rsid": input.reference.rsid,
        "performance_as_of": input.performance_as_of,
        "source_url": input.reference.url, "capture": capture::provenance(owned.capture),
        "raw_metadata_capture": capture::provenance(metadata), "locator": row.locator,
        "result_id": row.result_id, "disposition": "resource_limit", "unfinished": true,
        "requested_bytes": footprint.bytes, "requested_work": footprint.work,
        "byte_limit": budget::WINDOW_BYTES, "work_limit": budget::WINDOW_WORK,
        "parser_revision": receipt::PARSER_REVISION,
    });
    receipt::identified(input.reference, &payload).map(|key| (key, payload))
}

pub(super) fn summary(
    input: &Input<'_>,
    metadata: &FetchOutcome,
    window: &Window,
) -> CrawlResult<(String, serde_json::Value)> {
    let mut payload = receipt::context(input, metadata)?;
    let fields = payload.as_object_mut().ok_or_else(invalid)?;
    fields.insert(
        "disposition".into(),
        serde_json::json!(if window.complete {
            "projection_applied"
        } else {
            "partial"
        }),
    );
    fields.insert("projected_rows".into(), serde_json::json!(window.rows));
    fields.insert("windows".into(), serde_json::json!(window.ordinal));
    fields.insert(
        "projection_context_digest".into(),
        serde_json::json!(format!("{:x}", window.digest.clone().finalize())),
    );
    fields.insert("unfinished".into(), serde_json::json!(!window.complete));
    receipt::identified(input.reference, &payload).map(|key| (key, payload))
}

pub(super) fn blocked(
    input: &Input<'_>,
    metadata: &FetchOutcome,
    index: Option<usize>,
    error: &CrawlError,
) -> CrawlResult<(String, serde_json::Value)> {
    let locator = index.and_then(|index| {
        input
            .acquired
            .result_set(&input.reference.rsid, input.performance_as_of)?
            .page
            .rows
            .get(index)
    });
    let payload = serde_json::json!({
        "meet": input.reference.meet_id, "rsid": input.reference.rsid, "source_url": input.reference.url,
        "performance_as_of": input.performance_as_of,
        "capture": capture::provenance(&input.acquired.outcome.capture),
        "raw_metadata_capture": capture::provenance(metadata),
        "locator": locator.map(|row| row.locator.as_str()), "resume_from_owned_index": index,
        "disposition": "resource_limit", "unfinished": true, "resource_error": error.to_string(),
        "remaining_rows": "resume this result set from immutable owned capture; no final completion receipt",
    });
    receipt::identified(input.reference, &payload).map(|key| (key, payload))
}

pub(super) fn commit(
    ctx: &AdapterContext<'_>,
    entry: &(String, serde_json::Value),
) -> CrawlResult<()> {
    let mut batch = ctx.write_batch();
    super::super::super::journal_changed(ctx, &mut batch, &entry.0, &entry.1)?;
    batch.commit()
}

fn invalid() -> CrawlError {
    CrawlError::Invariant {
        detail: "result window receipt has no owned locator or object".into(),
    }
}
