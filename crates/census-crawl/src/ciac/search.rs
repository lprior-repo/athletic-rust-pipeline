use super::map::{school_entities, SchoolExtract};
use super::pages::parse_directory;
use super::{Options, HOST};
use crate::net::FetchOptions;
use crate::AdapterReport;
use crate::{AdapterContext, CrawlResult};

pub async fn resolve_schools(
    ctx: &AdapterContext<'_>,
    options: &Options,
    report: &mut AdapterReport,
) -> CrawlResult<Vec<SchoolExtract>> {
    let url = format!("{HOST}/Directory.aspx");
    let fetch_opts = FetchOptions {
        refresh: ctx.refresh || options.refresh,
        allow_not_found: false,
        headers: Vec::new(),
    };

    let outcome = ctx.fetcher.get(&url, &fetch_opts).await?;
    let html = String::from_utf8_lossy(&outcome.body);

    let parsed = parse_directory(&html);
    let observed_on = if options.observed_on.trim().is_empty() {
        ctx.observed_on.clone()
    } else {
        options.observed_on.clone()
    };

    let mut results = Vec::new();
    for (name, table) in &parsed {
        let extract = school_entities(name, table, &observed_on);
        results.push(extract);
    }

    let limit = options.limit.map_or(usize::MAX, |value| value);
    if results.len() > limit {
        results.truncate(limit);
    }

    report.note(format!(
        "parsed {} school tables from directory page",
        parsed.len()
    ));

    Ok(results)
}
