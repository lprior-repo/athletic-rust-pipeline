use crate::net::cache::CacheMeta;
use crate::net::{CapturedFetch, FetchOutcome};
use crate::participation_evidence::{bound, hash, invalid, json};
use crate::CrawlResult;
use census_domain::model::NormalizedObservedHttpV1;
use census_store::{PublicSchoolIntakeCaptureV1, StoreResult};

pub(super) struct ObservedCapture {
    pub(super) fetch: FetchOutcome,
    pub(super) observed: NormalizedObservedHttpV1,
}

pub(super) fn read(capture: &PublicSchoolIntakeCaptureV1) -> CrawlResult<ObservedCapture> {
    let fetch = materialize(capture)?;
    let observed =
        CapturedFetch::validate_observed_metadata(capture.prepared().metadata_bytes(), &fetch)?
            .ok_or_else(|| invalid("owned provider school normalized observation absent"))?;
    require_response(capture, &fetch, &observed)?;
    require_mime(&fetch, &observed)?;
    Ok(ObservedCapture { fetch, observed })
}

fn materialize(capture: &PublicSchoolIntakeCaptureV1) -> StoreResult<FetchOutcome> {
    let body = capture.body();
    bound(
        body.len(),
        8 * 1024 * 1024,
        "owned provider school body bytes",
    )?;
    bound(
        capture.prepared().metadata_bytes().len(),
        64 * 1024,
        "owned provider school metadata bytes",
    )?;
    let metadata: CacheMeta =
        serde_json::from_slice(capture.prepared().metadata_bytes()).map_err(json)?;
    if body.is_empty()
        || std::str::from_utf8(body).is_err()
        || capture.occurrence() != capture.body_reference()
        || !capture.work().request().observed_http_required()
        || metadata.method != "GET"
        || metadata.status != 200
        || metadata.url != capture.work().request().original_url()
        || metadata.content_digest != capture.prepared().content_digest().as_str()
        || metadata.content_digest != hash(body)
        || metadata.bytes != body.len()
        || u64::try_from(body.len()).map_err(|_| census_store::StoreError::CounterOverflow)?
            != capture.prepared().content_bytes()
    {
        return Err(invalid("owned provider school retained capture differs"));
    }
    let mut original = Vec::new();
    original
        .try_reserve_exact(body.len())
        .map_err(|_| invalid("owned provider school body allocation refused"))?;
    original.extend_from_slice(body);
    Ok(FetchOutcome {
        url: metadata.url,
        response_url: metadata.response_url,
        method: metadata.method,
        status: metadata.status,
        content_digest: metadata.content_digest,
        bytes: metadata.bytes,
        fetched_at: metadata.fetched_at,
        from_cache: true,
        content_type: metadata.content_type,
        body: original,
    })
}

fn require_response(
    capture: &PublicSchoolIntakeCaptureV1,
    fetch: &FetchOutcome,
    observed: &NormalizedObservedHttpV1,
) -> StoreResult<()> {
    let last = observed
        .hops()
        .last()
        .ok_or_else(|| invalid("owned provider school final observation absent"))?;
    if capture.response().status() != fetch.status
        || Some(capture.response().response_url()) != fetch.response_url.as_deref()
        || capture.response().observed_at() != fetch.fetched_at
        || capture.response().observed_http() != Some(last.response())
        || observed
            .hops()
            .iter()
            .any(|hop| hop.dispatch().attempt() != capture.occurrence().attempt())
    {
        return Err(invalid(
            "owned provider school response observation differs",
        ));
    }
    Ok(())
}

fn require_mime(fetch: &FetchOutcome, observed: &NormalizedObservedHttpV1) -> StoreResult<()> {
    let last = observed
        .hops()
        .last()
        .ok_or_else(|| invalid("owned provider school final observation absent"))?;
    let mut values = last
        .response()
        .headers()
        .iter()
        .filter(|header| header.name() == "content-type");
    let value = values
        .next()
        .ok_or_else(|| invalid("owned provider school observed MIME absent"))?;
    let value = std::str::from_utf8(value.value())
        .map_err(|_| invalid("owned provider school observed MIME is not UTF-8"))?;
    if values.next().is_some()
        || fetch.content_type.as_deref() != Some(value)
        || !(value.eq_ignore_ascii_case("text/html")
            || value.eq_ignore_ascii_case("text/html; charset=utf-8"))
    {
        return Err(invalid("owned provider school observed MIME differs"));
    }
    Ok(())
}
