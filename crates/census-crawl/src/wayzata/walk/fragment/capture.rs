use super::*;
use sha2::{Digest, Sha256};

const CHUNK_BYTES: usize = 64 * 1024;

#[derive(Serialize, Deserialize)]
pub(super) struct Capture {
    fetched: FetchOutcome,
    pub(super) rows: usize,
    label: String,
    chunks: usize,
}

pub(super) async fn freeze(
    ctx: &AdapterContext<'_>,
    options: &Options,
    sport: ScheduleSport,
    year: i16,
) -> CrawlResult<Capture> {
    let label = format!("wayzata-page/{year}/{sport:?}");
    if let Some(capture) = plan::read(ctx, &label)? {
        return Ok(capture);
    }
    let url = schedule_url(sport, year);
    let mut fetch_options = ctx.fetch_options();
    fetch_options.refresh = ctx.refresh || options.refresh;
    let mut fetched = ctx.fetcher.get(&url, &fetch_options).await?;
    let body = validate_page(&fetched, &url)?;
    let rows = super::super::super::parse::rows(body, year)?.len();
    budget::check(
        "Wayzata frozen page body",
        fetched.body.len(),
        crate::recording::MAX_RECORDED_BYTES,
    )?;
    let chunks = fetched.body.chunks(CHUNK_BYTES).enumerate().try_fold(
        0usize,
        |count, (ordinal, bytes)| {
            plan::write(ctx, &format!("{label}/{ordinal}"), &bytes)?;
            count
                .checked_add(1)
                .ok_or_else(|| budget::arithmetic("Wayzata frozen body chunks"))
        },
    )?;
    fetched.body = Vec::new();
    let capture = Capture {
        fetched,
        rows,
        label: label.clone(),
        chunks,
    };
    plan::write(ctx, &label, &capture)?;
    Ok(capture)
}

pub(super) fn read(ctx: &AdapterContext<'_>, capture: Capture) -> CrawlResult<FetchOutcome> {
    let Capture {
        mut fetched,
        label,
        chunks,
        ..
    } = capture;
    budget::check(
        "Wayzata frozen page bytes",
        fetched.bytes,
        crate::recording::MAX_RECORDED_BYTES,
    )?;
    budget::check(
        "Wayzata frozen page chunks",
        chunks,
        crate::recording::MAX_RECORDED_BYTES / CHUNK_BYTES,
    )?;
    fetched
        .body
        .try_reserve_exact(fetched.bytes)
        .map_err(|_| budget::arithmetic("Wayzata frozen body allocation"))?;
    for ordinal in 0..chunks {
        let part: Vec<u8> = plan::read(ctx, &format!("{label}/{ordinal}"))?
            .ok_or_else(|| budget::arithmetic("missing Wayzata frozen body chunk"))?;
        budget::check("Wayzata frozen chunk bytes", part.len(), CHUNK_BYTES)?;
        let bytes = fetched
            .body
            .len()
            .checked_add(part.len())
            .ok_or_else(|| budget::arithmetic("Wayzata frozen body extent"))?;
        budget::check("Wayzata frozen body extent", bytes, fetched.bytes)?;
        fetched.body.extend(part);
    }
    if fetched.body.len() != fetched.bytes
        || format!("{:x}", Sha256::digest(&fetched.body)) != fetched.content_digest
    {
        return Err(CrawlError::Invariant {
            detail: format!("Wayzata frozen capture changed: {label}"),
        });
    }
    Ok(fetched)
}
