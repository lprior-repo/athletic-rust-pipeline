use super::{
    parse_owned_meet, OwnedMeetOutcome, OwnedMeetVerdict, OWNED_CAPTURE_PHASE, OWNED_MEET_PHASE,
};
use crate::milesplit::fetch::{fetch_owned_capture, owned_meet_id, owned_meet_url};
use crate::milesplit::wire::ResultSetRef;
use crate::net::FetchOutcome;
use crate::{AdapterContext, CrawlError, CrawlResult};
use base64::{engine::general_purpose::STANDARD, Engine};

const CAPTURE_CHUNK_BYTES: usize = 32 * 1024;

#[tracing::instrument(skip(ctx))]
pub(crate) async fn read_owned_meet(
    ctx: &AdapterContext<'_>,
    reference: &ResultSetRef,
) -> CrawlResult<Option<String>> {
    let capture = match fetch_owned_capture(ctx.fetcher, reference, &ctx.fetch_options()).await {
        Ok(capture) => capture,
        Err(error) => return record_failure(ctx, reference, &error),
    };
    let key = content_key(reference, &capture);
    if capture.status == 200
        && ctx
            .store
            .journal_keys(OWNED_MEET_PHASE)?
            .contains(&format!("parsed/{key}"))
    {
        return Ok(None);
    }
    let verdict = if capture.status == 200 {
        parse_owned_meet(&capture.body, owned_meet_id(reference)?)
    } else {
        OwnedMeetVerdict::Refused {
            status: capture.status,
        }
    };
    record_outcome(ctx, reference, &OwnedMeetOutcome { capture, verdict })
}

fn content_key(reference: &ResultSetRef, capture: &FetchOutcome) -> String {
    format!("{}/{}", reference.meet_id, capture.content_digest)
}

fn record_outcome(
    ctx: &AdapterContext<'_>,
    reference: &ResultSetRef,
    outcome: &OwnedMeetOutcome,
) -> CrawlResult<Option<String>> {
    let key = content_key(reference, &outcome.capture);
    if !ctx
        .store
        .journal_keys(OWNED_CAPTURE_PHASE)?
        .contains(&format!("{key}/manifest"))
    {
        let mut capture_batch = ctx.write_batch();
        record_capture(&mut capture_batch, &key, &outcome.capture)?;
        capture_batch.commit()?;
    }
    let mut batch = ctx.write_batch();
    let (receipt, summary, failure) = match &outcome.verdict {
        OwnedMeetVerdict::Parsed(page) => record_parsed(&mut batch, &key, &outcome.capture, page)?,
        verdict => (
            format!("partial/{key}"),
            serde_json::json!({
                "capture": &outcome.capture, "verdict": verdict,
            }),
            Some(format!("{}: {verdict:?}", outcome.capture.url)),
        ),
    };
    batch.journal_done(OWNED_MEET_PHASE, &receipt, &summary)?;
    batch.commit()?;
    Ok(failure)
}

type RecordedOutcome = (String, serde_json::Value, Option<String>);

fn record_parsed(
    batch: &mut crate::recording::RowBatch<'_>,
    key: &str,
    capture: &FetchOutcome,
    page: &super::OwnedMeetPage,
) -> CrawlResult<RecordedOutcome> {
    page.rows.iter().try_for_each(|row| {
        batch.journal_done(OWNED_MEET_PHASE, &format!("row/{key}/{}", row.locator), row)
    })?;
    page.rejected.iter().try_for_each(|rejection| {
        batch.journal_done(
            OWNED_MEET_PHASE,
            &format!("rejected/{key}/{}", rejection.locator),
            rejection,
        )
    })?;
    let complete_parse = page.rejected.is_empty();
    let receipt = format!(
        "{}/{key}",
        if complete_parse { "parsed" } else { "partial" }
    );
    let summary = serde_json::json!({
        "disposition": if complete_parse { "parsed" } else { "partial" },
        "capture": capture, "published_rows": page.published_rows,
        "owned_rows": page.rows.len(), "rejected_rows": page.rejected.len(),
        "completeness": page.completeness, "ownership_complete": page.ownership_complete(),
    });
    let failure = (!complete_parse).then(|| {
        format!(
            "{}: partial owned-meet parse; {} rejected source rows",
            capture.url,
            page.rejected.len()
        )
    });
    Ok((receipt, summary, failure))
}

fn record_capture(
    batch: &mut crate::recording::RowBatch<'_>,
    key: &str,
    capture: &FetchOutcome,
) -> CrawlResult<()> {
    capture
        .body
        .chunks(CAPTURE_CHUNK_BYTES)
        .enumerate()
        .try_for_each(|(index, bytes)| {
            batch.journal_done(
                OWNED_CAPTURE_PHASE,
                &format!("{key}/{index}"),
                &serde_json::json!({
                    "capture": capture, "chunk_index": index, "raw_base64": STANDARD.encode(bytes),
                }),
            )
        })?;
    batch.journal_done(
        OWNED_CAPTURE_PHASE,
        &format!("{key}/manifest"),
        &serde_json::json!({
            "capture": capture, "chunk_bytes": CAPTURE_CHUNK_BYTES,
            "encoding": "base64",
            "chunks": capture.body.chunks(CAPTURE_CHUNK_BYTES).len(),
        }),
    )
}

fn record_failure(
    ctx: &AdapterContext<'_>,
    reference: &ResultSetRef,
    error: &CrawlError,
) -> CrawlResult<Option<String>> {
    let url = owned_meet_url(reference)?;
    let reason = format!("{url}: {error}");
    let mut batch = ctx.write_batch();
    let disposition = match error {
        CrawlError::Fetch(
            crate::net::FetchError::Http {
                status: 401 | 403, ..
            }
            | crate::net::FetchError::Policy { .. }
            | crate::net::FetchError::BrowserLane {
                retryable: false, ..
            },
        ) => "refused",
        _ => "failed",
    };
    batch.journal_done(
        OWNED_MEET_PHASE,
        &format!("{disposition}/{}", reference.meet_id),
        &serde_json::json!({
            "source_url": url, "disposition": disposition, "error": error.to_string(),
            "capture_available": false,
        }),
    )?;
    batch.commit()?;
    Ok(Some(reason))
}

#[cfg(test)]
mod tests;
