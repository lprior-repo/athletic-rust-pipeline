use super::{
    parse_owned_meet, OwnedMeetOutcome, OwnedMeetVerdict, OWNED_CAPTURE_PHASE, OWNED_MEET_PHASE,
};
use crate::milesplit::fetch::{fetch_owned_capture, owned_meet_id, owned_meet_url};
use crate::milesplit::wire::ResultSetRef;
use crate::net::FetchOutcome;
use crate::{AdapterContext, CrawlError, CrawlResult};
#[cfg(test)]
use base64::{engine::general_purpose::STANDARD, Engine};

const CAPTURE_CHUNK_BYTES: usize = 32 * 1024;

mod capture;
mod interpretation;
mod journal;

#[cfg(test)]
use capture::record_capture;

pub(crate) fn acquisition_manifest_key(
    meet_id: u64,
    capture: &crate::net::FetchOutcome,
) -> CrawlResult<String> {
    capture::acquisition_manifest_key(meet_id, capture)
}

#[tracing::instrument(skip(ctx))]
pub(crate) async fn read_owned_meet(
    ctx: &AdapterContext<'_>,
    reference: &ResultSetRef,
) -> CrawlResult<OwnedMeetOutcome> {
    let capture = match fetch_owned_capture(ctx.fetcher, reference, &ctx.fetch_options()).await {
        Ok(capture) => capture,
        Err(error) => {
            record_failure(ctx, reference, &error)?;
            return Err(error);
        }
    };
    let verdict = if capture.status == 200 {
        parse_owned_meet(&capture.body, owned_meet_id(reference)?)
    } else {
        OwnedMeetVerdict::Refused {
            status: capture.status,
        }
    };
    let outcome = OwnedMeetOutcome { capture, verdict };
    record_outcome(ctx, reference, &outcome).map(|_| outcome)
}

fn content_key(reference: &ResultSetRef, capture: &FetchOutcome) -> String {
    format!("{}/{}", reference.meet_id, capture.content_digest)
}

fn record_outcome(
    ctx: &AdapterContext<'_>,
    reference: &ResultSetRef,
    outcome: &OwnedMeetOutcome,
) -> CrawlResult<()> {
    let key = content_key(reference, &outcome.capture);
    let manifest = acquisition_manifest_key(owned_meet_id(reference)?, &outcome.capture)?;
    capture::archive(ctx, &key, &manifest, &outcome.capture)?;
    interpretation::record(ctx, &key, &manifest, outcome)
}

fn record_failure(
    ctx: &AdapterContext<'_>,
    reference: &ResultSetRef,
    error: &CrawlError,
) -> CrawlResult<()> {
    let url = owned_meet_url(reference)?;
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
    Ok(())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod acquisition_partial_tests;
