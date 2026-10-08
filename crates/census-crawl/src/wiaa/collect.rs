use super::parse::parse_directory_letter;
use super::{fetch_options, letters_for, Options, HOST, INDEX_PATH};
use crate::directory::acquisition::{fail, owe, text};
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::UsJurisdiction;
use futures::{stream, StreamExt, TryStreamExt};

#[path = "collect_schools.rs"]
mod collect_schools;

pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let before = ctx.fetcher.stats().await;
    let report = AdapterReport::new("wiaa", "schools");
    if !options.states.is_empty() && !options.states.contains(&UsJurisdiction::Wisconsin) {
        return Ok(report);
    }
    let letters = letters_for(&options.school_names);
    let (mut report, _) = stream::iter(&letters)
        .map(Ok::<_, CrawlError>)
        .try_fold(
            (report, 0usize),
            |(mut report, mut attempted), letter| async move {
                collect_letter(ctx, options, *letter, (&mut report, &mut attempted)).await?;
                Ok((report, attempted))
            },
        )
        .await?;
    if letters.is_empty() {
        owe(&mut report, format!("{HOST}{INDEX_PATH}"))?;
    }
    if report.unfinished.is_empty() {
        report.finish_frontier();
    }
    let after = ctx.fetcher.stats().await;
    report.requests = after
        .physical_requests()
        .saturating_sub(before.physical_requests());
    report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
    Ok(report)
}

async fn collect_letter(
    ctx: &AdapterContext<'_>,
    options: &Options,
    letter: char,
    progress: (&mut AdapterReport, &mut usize),
) -> CrawlResult<()> {
    let (report, attempted) = progress;
    let url = format!("{HOST}{INDEX_PATH}?LetterBtn={letter}");
    let capture = match ctx.fetcher.get(&url, &fetch_options(ctx, options)).await {
        Ok(capture) if capture.status == 200 => capture,
        Ok(capture) => return fail(report, &url, format!("HTTP {}", capture.status)),
        Err(error) => return fail(report, &url, error),
    };
    let body = match text(&capture) {
        Ok(body) => body,
        Err(error) => return fail(report, &url, error),
    };
    let rows = parse_directory_letter(body);
    if rows.is_empty() {
        owe(report, &url)?;
    }
    if !body.contains("</html>") {
        owe(report, &url)?;
    }
    stream::iter(&rows)
        .map(Ok::<_, CrawlError>)
        .try_fold((report, attempted), |(report, attempted), row| async move {
            collect_schools::process_school(ctx, options, row, (report, attempted)).await?;
            Ok((report, attempted))
        })
        .await
        .map(|_| ())
}
