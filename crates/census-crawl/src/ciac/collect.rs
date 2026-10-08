use super::map::SchoolExtract;
use super::search::collect_directory;
use super::{Options, ASSOCIATION};
use crate::directory::acquisition::{publish as persist, publish_school as school};
use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::model::SourceNamespace;
use census_domain::UsJurisdiction;
use census_store::Table;

pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let mut report = AdapterReport::new("ciac", "schools");
    if !options.states.is_empty() && !options.states.contains(&UsJurisdiction::Connecticut) {
        return Ok(report);
    }
    let before = ctx.fetcher.stats().await;
    collect_directory(ctx, options, &mut report).await?;
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

pub(super) fn emit_school(
    ctx: &AdapterContext<'_>,
    extract: &SchoolExtract,
    report: &mut AdapterReport,
) -> CrawlResult<()> {
    let stamp = extract
        .school
        .evidence
        .first()
        .map_or(ctx.observed_on.as_str(), |evidence| {
            evidence.observed_on.as_str()
        });
    let locator = format!("{}#school={}", super::HOST, extract.school.id);
    let written = school(
        ctx,
        ("ciac", &locator),
        (
            &SourceNamespace::association_school(ASSOCIATION),
            &extract.school,
            stamp,
        ),
        report,
    )?;
    persist(
        ctx,
        ("ciac", &locator),
        Table::Coaches,
        &extract.coaches,
        report,
    )?;
    report.rows = report
        .rows
        .saturating_add(u64::try_from(written).map_or(u64::MAX, |value| value));
    Ok(())
}
