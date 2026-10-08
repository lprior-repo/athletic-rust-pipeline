use super::map::{school_entities, SearchResult};
use super::pages::parse_ad_page;
use super::Options;
use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::UsJurisdiction;
use futures::{stream, StreamExt, TryStreamExt};
use std::collections::HashSet;

mod fetch;
mod search;
mod write;

use fetch::{fetch_page, PageKind, SchoolPages};
use search::resolve_schools;
use write::{emit_school, persist_failure, CAPTURE_JOURNAL};

#[tracing::instrument(skip(ctx, options))]
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let mut report = AdapterReport::new("ohsaa", "schools");
    let evaluated_on = if options.observed_on.trim().is_empty() {
        &ctx.observed_on
    } else {
        &options.observed_on
    };
    let before = ctx.fetcher.stats().await;
    if !options.states.is_empty() && !options.states.contains(&UsJurisdiction::Ohio) {
        let codes: Vec<&str> = options.states.iter().map(|state| state.code()).collect();
        report.note(format!(
            "states {codes:?} do not include OH; this adapter covers Ohio only"
        ));
        return Ok(report);
    }
    let to_process = resolve_schools(ctx, options, &mut report).await?;
    let tally = Tally {
        captures: ctx.store.journal_keys(CAPTURE_JOURNAL)?,
        completed: ctx.store.journal_keys("ohsaa_schools")?,
        ..Tally::default()
    };
    let (mut report, tally) = stream::iter(&to_process)
        .map(Ok::<_, crate::CrawlError>)
        .try_fold((report, tally), |(mut report, mut tally), sr| async move {
            process_school(ctx, sr, evaluated_on, &mut report, &mut tally).await?;
            Ok((report, tally))
        })
        .await?;
    let after = ctx.fetcher.stats().await;
    report.requests = after
        .physical_requests()
        .saturating_sub(before.physical_requests());
    report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
    report.with_email = tally.with_email;
    report.note(format!(
        "processed {} of {} requested schools ({} not_found, {} fetch_failures, {} coach_rows, {} office_roles_skipped)",
        tally.processed, to_process.len(), tally.not_found, tally.fetch_failures,
        tally.coach_rows, tally.office_roles_skipped
    ));
    Ok(report)
}

#[derive(Default)]
struct Tally {
    processed: usize,
    not_found: usize,
    fetch_failures: usize,
    coach_rows: usize,
    with_email: u64,
    office_roles_skipped: usize,
    captures: HashSet<String>,
    completed: HashSet<String>,
}

#[tracing::instrument(skip(ctx, sr, report, tally))]
async fn process_school(
    ctx: &AdapterContext<'_>,
    sr: &SearchResult,
    evaluated_on: &str,
    report: &mut AdapterReport,
    tally: &mut Tally,
) -> CrawlResult<()> {
    let sports = match fetch_page(ctx, sr, PageKind::Sports).await {
        Ok(capture) => capture,
        Err(failure) => {
            failure.report(sr, "sports", report, tally);
            persist_failure(ctx, sr, "sports", &failure, evaluated_on)?;
            return Ok(());
        }
    };
    let ad = fetch_page(ctx, sr, PageKind::AthleticDirector).await;
    if let Err(failure) = &ad {
        failure.report(sr, "AD", report, tally);
    }
    let pages = SchoolPages { sports, ad };
    let extract = school_entities(sr, &pages.sports, pages.ad.as_ref().ok());
    if let Ok(capture) = &pages.ad {
        let parsed = parse_ad_page(&String::from_utf8_lossy(&capture.body));
        tally.office_roles_skipped = tally
            .office_roles_skipped
            .saturating_add(parsed.office_roles.len());
    }
    emit_school(ctx, sr, &pages, &extract, evaluated_on, report, tally)?;
    tally.processed = tally.processed.saturating_add(1);
    Ok(())
}
