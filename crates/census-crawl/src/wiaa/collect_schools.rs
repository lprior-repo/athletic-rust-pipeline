use super::super::map::school_entities;
use super::super::parse::{parse_school_page, IndexEntry};
use super::super::{fetch_options, Options, ASSOCIATION};
use crate::directory::acquisition::{
    fail, owe, publish as persist, publish_school as school, text,
};
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::{normalize_name, SourceNamespace};
use census_store::Table;

pub(super) async fn process_school(
    ctx: &AdapterContext<'_>,
    options: &Options,
    entry: &IndexEntry,
    progress: (&mut AdapterReport, &mut usize),
) -> CrawlResult<()> {
    let (report, attempted) = progress;
    if !options.school_names.is_empty()
        && !options
            .school_names
            .iter()
            .any(|name| normalize_name(name) == normalize_name(&entry.name))
    {
        return Ok(());
    }
    let url = entry.page_url();
    if options.limit.is_some_and(|limit| *attempted >= limit) {
        return owe(report, url);
    }
    *attempted = attempted
        .checked_add(1)
        .ok_or_else(|| CrawlError::Arithmetic {
            detail: "school counter".into(),
        })?;
    let capture = match ctx.fetcher.get(&url, &fetch_options(ctx, options)).await {
        Ok(capture) if capture.status == 200 => capture,
        Ok(capture) => return fail(report, &url, format!("HTTP {}", capture.status)),
        Err(error) => return fail(report, &url, error),
    };
    let body = match text(&capture) {
        Ok(body) => body,
        Err(error) => return fail(report, &url, error),
    };
    let page = parse_school_page(body);
    if page.name.trim().is_empty() || normalize_name(&page.name) != normalize_name(&entry.name) {
        return fail(report, &url, "missing or foreign school owner");
    }
    let Some(extract) = school_entities(entry, &page, &capture.fetched_at) else {
        return fail(report, &url, "missing school owner");
    };
    persist_school(ctx, &url, &extract, &capture.fetched_at, report)
}

fn persist_school(
    ctx: &AdapterContext<'_>,
    url: &str,
    extract: &super::super::map::SchoolExtract,
    observed_on: &str,
    report: &mut AdapterReport,
) -> CrawlResult<()> {
    let written = school(
        ctx,
        ("wiaa", url),
        (
            &SourceNamespace::association_school(ASSOCIATION),
            &extract.school,
            observed_on,
        ),
        report,
    )?;
    persist(ctx, ("wiaa", url), Table::Coaches, &extract.coaches, report)?;
    report.rows = report
        .rows
        .saturating_add(u64::try_from(written).map_err(|_| CrawlError::Arithmetic {
            detail: "school count".into(),
        })?);
    Ok(())
}
