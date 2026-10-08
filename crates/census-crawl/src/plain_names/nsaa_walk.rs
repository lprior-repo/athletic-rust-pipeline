use super::nsaa::{nsaa_school_url, parse_nsaa_directory, NsaaSchool};
use super::nsaa_coaches::{nsaa_coaches, parse_nsaa_school};
use crate::directory::acquisition::{fail, publish as persist, publish_school as school, text};
use crate::net::FetchOptions;
use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::model::SourceNamespace;
use census_store::Table;

pub(super) async fn visit(
    ctx: &AdapterContext<'_>,
    fetch: &FetchOptions,
    report: &mut AdapterReport,
    name: &str,
) -> CrawlResult<(usize, usize)> {
    let url = nsaa_school_url(name);
    let capture = match ctx.fetcher.get(&url, fetch).await {
        Ok(capture) => capture,
        Err(error) => {
            fail(report, &url, error)?;
            return Ok((0, 0));
        }
    };
    let blocks = match text(&capture).and_then(parse_nsaa_directory) {
        Ok(blocks) => blocks,
        Err(error) => {
            fail(report, &url, error)?;
            return Ok((0, 0));
        }
    };
    let Some(entry) = blocks.iter().find(|entry| entry.name == name) else {
        fail(report, &url, "requested school block is absent")?;
        return Ok((0, 0));
    };
    project(ctx, report, (&url, &capture.fetched_at), entry)
}

fn project(
    ctx: &AdapterContext<'_>,
    report: &mut AdapterReport,
    source: (&str, &str),
    entry: &NsaaSchool,
) -> CrawlResult<(usize, usize)> {
    let (canonical, id) = parse_nsaa_school(entry, source.0, source.1);
    let written = school(
        ctx,
        (super::NSAA_ADAPTER_ID, source.0),
        (
            &SourceNamespace::association_school(super::NSAA_ADAPTER_ID),
            &canonical,
            source.1,
        ),
        report,
    )?;
    let coaches = entry
        .roles
        .iter()
        .enumerate()
        .try_fold(0usize, |written, (ordinal, role)| {
            let locator = format!("{}#role={ordinal}", source.0);
            let selected = NsaaSchool {
                name: entry.name.clone(),
                city: entry.city.clone(),
                enrollment: entry.enrollment,
                homepage: entry.homepage.clone(),
                roles: vec![role.clone()],
            };
            let rows = match nsaa_coaches(&selected, &id, source.0, source.1) {
                Ok(rows) => rows,
                Err(error) => {
                    fail(report, &locator, error)?;
                    return Ok(written);
                }
            };
            persist(
                ctx,
                (super::NSAA_ADAPTER_ID, &locator),
                Table::Coaches,
                &rows,
                report,
            )
            .map(|rows| written.saturating_add(rows))
        })?;
    Ok((written, coaches))
}
