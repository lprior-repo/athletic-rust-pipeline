use super::map::school_entities;
use super::pages::parse_directory;
use super::{Options, HOST};
use crate::directory::acquisition::{fail, owe, text};
use crate::{AdapterContext, AdapterReport, CrawlResult};

pub(super) async fn collect_directory(
    ctx: &AdapterContext<'_>,
    options: &Options,
    report: &mut AdapterReport,
) -> CrawlResult<()> {
    let url = format!("{HOST}/Directory.aspx?SchoolLevelID=1");
    let fetch = crate::net::FetchOptions {
        refresh: ctx.refresh || options.refresh,
        ..ctx.fetch_options()
    };
    let capture = match ctx.fetcher.get(&url, &fetch).await {
        Ok(capture) => capture,
        Err(error) => return fail(report, &url, error),
    };
    let body = match text(&capture) {
        Ok(body) => body,
        Err(error) => return fail(report, &url, error),
    };
    let parsed = parse_directory(body);
    if parsed.is_empty()
        || body.matches("<table class='DirectoryStaffTable'>").count() != parsed.len()
    {
        owe(report, &url)?;
    }
    parsed.iter().try_for_each(|(name, table)| {
        if options.limit.is_some_and(|limit| {
            report.rows >= u64::try_from(limit).map_or(u64::MAX, |value| value)
        }) {
            return owe(report, format!("{url}#school={name}"));
        }
        super::collect::emit_school(
            ctx,
            &school_entities(name, table, &capture.fetched_at),
            report,
        )
    })
}
