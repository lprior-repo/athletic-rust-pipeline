use super::map::{captured_school_entities, ParsedSchool};
use super::pages::parse_directory;
use super::{Options, ASSOCIATION, HOST_WWW};
use crate::directory::acquisition::{
    fail, owe, publish as persist, publish_school as school, text,
};
use crate::net::FetchOutcome;
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::{normalize_name, SourceNamespace};
use census_domain::UsJurisdiction;
use census_store::Table;
use futures::{stream, StreamExt, TryStreamExt};

pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let before = ctx.fetcher.stats().await;
    let mut report = AdapterReport::new("mpa", "schools");
    if !options.states.is_empty() && !options.states.contains(&UsJurisdiction::Maine) {
        return Ok(report);
    }
    let url = format!("{HOST_WWW}/SchoolPages/School.aspx");
    if let Some(directory) = fetch(ctx, &url, &mut report).await? {
        let body = match text(&directory) {
            Ok(body) => body,
            Err(error) => {
                fail(&mut report, &url, error)?;
                ""
            }
        };
        let entries = parse_directory(body);
        if entries.is_empty() {
            owe(&mut report, &url)?;
        }
        report = stream::iter(&entries)
            .map(Ok::<_, CrawlError>)
            .try_fold(report, |mut report, entry| {
                let directory = &directory;
                async move {
                    process_school(ctx, options, entry, directory, &mut report).await?;
                    Ok(report)
                }
            })
            .await?;
        if report.unfinished.is_empty() {
            report.finish_frontier();
        }
    }
    let after = ctx.fetcher.stats().await;
    report.requests = after
        .physical_requests()
        .saturating_sub(before.physical_requests());
    report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
    Ok(report)
}

async fn fetch(
    ctx: &AdapterContext<'_>,
    url: &str,
    report: &mut AdapterReport,
) -> CrawlResult<Option<FetchOutcome>> {
    match ctx.fetcher.get(url, &ctx.fetch_options()).await {
        Ok(capture) if capture.status == 200 => Ok(Some(capture)),
        Ok(capture) => {
            fail(report, url, format!("HTTP {}", capture.status))?;
            Ok(None)
        }
        Err(error) => {
            fail(report, url, error)?;
            Ok(None)
        }
    }
}

async fn process_school(
    ctx: &AdapterContext<'_>,
    options: &Options,
    entry: &super::pages::SchoolEntry,
    directory: &FetchOutcome,
    report: &mut AdapterReport,
) -> CrawlResult<()> {
    if !options.school_names.is_empty()
        && !options
            .school_names
            .iter()
            .any(|name| normalize_name(name) == normalize_name(&entry.name))
    {
        return Ok(());
    }
    let url = format!(
        "{HOST_WWW}/SchoolPages/School.aspx?SchoolID={}&tab=staff",
        entry.school_id
    );
    if options
        .limit
        .is_some_and(|limit| report.rows >= u64::try_from(limit).map_or(u64::MAX, |value| value))
    {
        return owe(report, url);
    }
    let Some(staff) = fetch(ctx, &url, report).await? else {
        return Ok(());
    };
    let body = match text(&staff) {
        Ok(body) => body,
        Err(error) => return fail(report, &url, error),
    };
    if !body.contains("<table class='DirectoryStaffTable'>") || !body.contains("</table>") {
        owe(report, &url)?;
    }
    let parsed = ParsedSchool {
        name: entry.name.clone(),
        school_id: entry.school_id.clone(),
    };
    let extract = captured_school_entities(
        &parsed,
        &super::pages::parse_staff_table(body),
        directory,
        &staff,
    );
    persist_school(ctx, &url, &extract, &directory.fetched_at, report)
}

fn persist_school(
    ctx: &AdapterContext<'_>,
    url: &str,
    extract: &super::map::SchoolExtract,
    observed_on: &str,
    report: &mut AdapterReport,
) -> CrawlResult<()> {
    let written = school(
        ctx,
        ("mpa", url),
        (
            &SourceNamespace::association_school(ASSOCIATION),
            &extract.school,
            observed_on,
        ),
        report,
    )?;
    persist(ctx, ("mpa", url), Table::Coaches, &extract.coaches, report)?;
    report.rows = report
        .rows
        .saturating_add(u64::try_from(written).map_err(|_| CrawlError::Arithmetic {
            detail: "school count".into(),
        })?);
    Ok(())
}
