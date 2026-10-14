use super::{
    parse_owned_meet, OwnedCaptureResourceFailure, OwnedMeetOutcome, OwnedMeetParseError,
    OwnedMeetVerdict, OwnedParseResourceFailure, OwnedResourceReceipt, OWNED_CAPTURE_PHASE,
    OWNED_MEET_PHASE,
};
use crate::milesplit::fetch::{fetch_owned_capture, owned_meet_id, owned_meet_url};
use crate::milesplit::wire::ResultSetRef;
use crate::net::FetchOutcome;
use crate::{AdapterContext, CrawlError, CrawlResult};
#[cfg(test)]
use base64::{engine::general_purpose::STANDARD, Engine};

pub(super) const CAPTURE_CHUNK_BYTES: usize = 32 * 1024;

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
            if !error.retryable() {
                record_failure(ctx, reference, &error)?;
            }
            return Err(error);
        }
    };
    let verdict = if capture.status == 200 {
        match parse_owned_meet(&capture.body, owned_meet_id(reference)?) {
            Ok(page) => OwnedMeetVerdict::Parsed(page),
            Err(OwnedMeetParseError::Malformed(failure)) => OwnedMeetVerdict::Malformed {
                detail: failure.to_string(),
            },
            Err(OwnedMeetParseError::Resource(failure)) => {
                return retain_limited_capture(ctx, reference, capture, failure.into());
            }
        }
    } else {
        OwnedMeetVerdict::Refused {
            status: capture.status,
        }
    };
    let outcome = OwnedMeetOutcome { capture, verdict };
    match record_outcome(ctx, reference, &outcome) {
        Ok(()) => Ok(outcome),
        Err(error @ CrawlError::Resource { .. }) => {
            retain_limited_capture(ctx, reference, outcome.capture, error)
        }
        Err(error) => Err(error),
    }
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

fn retain_limited_capture(
    ctx: &AdapterContext<'_>,
    reference: &ResultSetRef,
    capture: FetchOutcome,
    error: CrawlError,
) -> CrawlResult<OwnedMeetOutcome> {
    let key = content_key(reference, &capture);
    let manifest = acquisition_manifest_key(owned_meet_id(reference)?, &capture)?;
    match capture::archive(ctx, &key, &manifest, &capture) {
        Ok(()) | Err(CrawlError::Resource { .. }) => {}
        Err(error) => return Err(error),
    }
    let archived = immutable_body(ctx.fetcher, &capture)?;
    let locator = match &error {
        CrawlError::OwnedParseResource(failure) => failure.locator().to_string(),
        _ => "data".into(),
    };
    let origin = match &error {
        CrawlError::OwnedParseResource(failure) => failure.stable_origin().to_string(),
        _ => error.to_string(),
    };
    let body_locator = format!("archive/bodies/{}.body", capture.content_digest);
    let payload = serde_json::json!({
        "capture": &capture, "disposition": "resource_limit", "unfinished": true,
        "capture_available": true, "immutable_body": body_locator, "unfinished_locator": locator,
        "resource_error": &origin,
    });
    let key = format!(
        "partial/resource-v2/{}/{}",
        reference.meet_id,
        journal::digest(&(manifest, &origin))?
    );
    let mut batch = ctx.write_batch();
    let staged = journal::stage_capture(ctx, &mut batch, OWNED_MEET_PHASE, &key, &payload)
        .and_then(|changed| if changed { batch.commit() } else { Ok(()) });
    let receipt = match staged {
        Ok(()) => OwnedResourceReceipt::Retained,
        Err(CrawlError::Resource { .. }) => OwnedResourceReceipt::ReportOnly,
        Err(error) => return Err(error),
    };
    Err(match error {
        CrawlError::OwnedParseResource(failure) => {
            failure.retain(capture, archived, receipt).into()
        }
        CrawlError::Resource {
            resource,
            requested,
            limit,
        } => {
            OwnedCaptureResourceFailure::new(resource, requested, limit, capture, archived, receipt)
                .into()
        }
        error => error,
    })
}

pub(crate) fn retained_parse_resource(
    fetcher: &crate::net::Fetcher,
    capture: FetchOutcome,
    failure: OwnedParseResourceFailure,
) -> CrawlResult<OwnedParseResourceFailure> {
    let archived = immutable_body(fetcher, &capture)?;
    Ok(failure.retain(capture, archived, OwnedResourceReceipt::NotRequested))
}

fn immutable_body(
    fetcher: &crate::net::Fetcher,
    capture: &FetchOutcome,
) -> CrawlResult<std::path::PathBuf> {
    let path = fetcher
        .cache_dir()
        .join("archive")
        .join("bodies")
        .join(format!("{}.body", capture.content_digest));
    let metadata = std::fs::metadata(&path).map_err(|source| CrawlError::Io {
        path: path.clone(),
        source,
    })?;
    let bytes = u64::try_from(capture.bytes).map_err(|_| CrawlError::Arithmetic {
        detail: "owned capture length exceeds archive metadata range".into(),
    })?;
    if !metadata.is_file() || metadata.len() != bytes {
        return Err(CrawlError::Invariant {
            detail: format!(
                "immutable owned capture {} is unavailable or has the wrong length",
                path.display()
            ),
        });
    }
    Ok(path)
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

#[cfg(test)]
mod retained_tests;
