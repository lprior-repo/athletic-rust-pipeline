use super::map::{school_entities, SearchResult};
use super::pages::parse_ad_page;
use super::Options;
use crate::{AdapterContext, AdapterReport, CollectionDisposition, CrawlError, CrawlResult};
use census_domain::UsJurisdiction;
use futures::{stream, StreamExt, TryStreamExt};

mod appointments;
mod fetch;
mod search;
mod write;
use fetch::{fetch_page, PageKind, SchoolPages};
use search::resolve_schools;
use write::{emit_school, persist_failure, Projection};

pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let mut report = AdapterReport::new("ohsaa", "schools");
    if !options.states.is_empty() && !options.states.contains(&UsJurisdiction::Ohio) {
        return Ok(report);
    }
    let before = ctx.fetcher.stats().await;
    let schools = resolve_schools(ctx, options, &mut report).await?;
    let evaluated_on = if options.observed_on.trim().is_empty() {
        &ctx.observed_on
    } else {
        &options.observed_on
    };
    let (mut report, tally) = walk(ctx, &schools, evaluated_on, report).await?;
    let after = ctx.fetcher.stats().await;
    report.requests = after
        .physical_requests()
        .checked_sub(before.physical_requests())
        .ok_or_else(counter_error)?;
    report.from_cache = after
        .cache_hits
        .checked_sub(before.cache_hits)
        .ok_or_else(counter_error)?;
    report.with_email = tally.with_email;
    report.note(format!("processed {} configured schools; {} fetch failures; {} coach rows; {} office roles retained outside coaching projection", tally.processed, tally.fetch_failures, tally.coach_rows, tally.office_roles_skipped));
    if !schools.is_empty() {
        report.finish_frontier();
    }
    Ok(report)
}

async fn walk(
    ctx: &AdapterContext<'_>,
    schools: &[SearchResult],
    evaluated_on: &str,
    report: AdapterReport,
) -> CrawlResult<(AdapterReport, Tally)> {
    stream::iter(schools)
        .map(Ok::<_, CrawlError>)
        .try_fold(
            (report, Tally::default()),
            |(mut report, mut tally), school| async move {
                process_school(ctx, school, evaluated_on, &mut report, &mut tally).await?;
                Ok((report, tally))
            },
        )
        .await
}

#[derive(Default)]
struct Tally {
    processed: usize,
    not_found: usize,
    fetch_failures: usize,
    coach_rows: usize,
    with_email: u64,
    office_roles_skipped: usize,
}

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
            failure.report(sr, "sports", report, tally)?;
            persist_failure(ctx, sr, "sports", &failure, evaluated_on)?;
            owe(report, &sr.ad_url())?;
            return Ok(());
        }
    };
    let ad = fetch_page(ctx, sr, PageKind::AthleticDirector).await;
    if let Err(failure) = &ad {
        failure.report(sr, "AD", report, tally)?;
    }
    let pages = SchoolPages { sports, ad };
    let mut extract = school_entities(sr, &pages.sports, pages.ad.as_ref().ok());
    if let Ok(capture) = &pages.ad {
        let html = std::str::from_utf8(&capture.body).map_err(|error| CrawlError::Schema {
            url: capture.url.clone(),
            detail: error.to_string(),
        })?;
        tally.office_roles_skipped = tally
            .office_roles_skipped
            .checked_add(parse_ad_page(html).office_roles.len())
            .ok_or_else(counter_error)?;
    }
    emit_school(
        ctx,
        Projection {
            school: sr,
            pages: &pages,
            extract: &mut extract,
        },
        evaluated_on,
        report,
        tally,
    )?;
    tally.processed = tally.processed.checked_add(1).ok_or_else(counter_error)?;
    Ok(())
}

fn owe(report: &mut AdapterReport, locator: &str) -> CrawlResult<()> {
    if report.unfinished.iter().any(|value| value == locator) {
        return Ok(());
    }
    report
        .unfinished
        .try_reserve(1)
        .map_err(|_| CrawlError::Resource {
            resource: "Ohio unfinished locators",
            requested: 1,
            limit: 4096,
        })?;
    report.unfinished.push(locator.to_string());
    report.disposition = CollectionDisposition::Partial;
    Ok(())
}

fn fail(report: &mut AdapterReport, locator: &str, detail: &str) -> CrawlResult<()> {
    report.errors = report.errors.checked_add(1).ok_or_else(counter_error)?;
    owe(report, locator)?;
    if report.errors <= 5 {
        report.note(detail.chars().take(4096).collect::<String>());
    }
    Ok(())
}

fn counter_error() -> CrawlError {
    CrawlError::Arithmetic {
        detail: "Ohio acquisition accounting overflow".to_string(),
    }
}
