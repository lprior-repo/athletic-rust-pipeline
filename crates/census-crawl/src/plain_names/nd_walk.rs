use super::nd::{parse_nd_offerings, parse_nd_school_page, parse_nd_staff, NdSchoolRef};
use super::nd_coaches::{nd_ad_coaches, nd_sport_coaches};
use crate::directory::acquisition::{fail, publish as persist, publish_school as school, text};
use crate::net::FetchOptions;
use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::model::SourceNamespace;
use census_store::Table;

pub(super) async fn visit(
    ctx: &AdapterContext<'_>,
    fetch: &FetchOptions,
    report: &mut AdapterReport,
    member: &NdSchoolRef,
) -> CrawlResult<(usize, usize)> {
    let url = member.url();
    let capture = match ctx.fetcher.get(&url, fetch).await {
        Ok(capture) => capture,
        Err(error) => {
            fail(report, &url, error)?;
            return Ok((0, 0));
        }
    };
    let body = match text(&capture) {
        Ok(body) => body,
        Err(error) => {
            fail(report, &url, error)?;
            return Ok((0, 0));
        }
    };
    let canonical = match parse_nd_school_page(body, member, &capture.fetched_at) {
        Ok(Some(canonical)) => canonical,
        Ok(None) => {
            fail(report, &url, "missing school heading")?;
            return Ok((0, 0));
        }
        Err(error) => {
            fail(report, &url, error)?;
            return Ok((0, 0));
        }
    };
    let written = school(
        ctx,
        (super::ND_ADAPTER_ID, &url),
        (
            &SourceNamespace::association_school(super::ND_ADAPTER_ID),
            &canonical.0,
            &capture.fetched_at,
        ),
        report,
    )?;
    let coaches = project(ctx, report, (&url, &capture.fetched_at), body, &canonical.1)?;
    Ok((written, coaches))
}

fn project(
    ctx: &AdapterContext<'_>,
    report: &mut AdapterReport,
    source: (&str, &str),
    body: &str,
    id: &census_domain::model::SchoolId,
) -> CrawlResult<usize> {
    let staff = match parse_nd_staff(body) {
        Ok(staff) => staff,
        Err(error) => {
            fail(report, source.0, error)?;
            return Ok(0);
        }
    };
    let ads = staff
        .iter()
        .enumerate()
        .try_fold(0usize, |written, (ordinal, entry)| {
            let locator = format!("{}#staff={ordinal}", source.0);
            let rows = nd_ad_coaches(std::slice::from_ref(entry), id, source.0, source.1);
            persist(
                ctx,
                (super::ND_ADAPTER_ID, &locator),
                Table::Coaches,
                &rows,
                report,
            )
            .map(|rows| written.saturating_add(rows))
        })?;
    let offerings = match parse_nd_offerings(body) {
        Ok(offerings) => offerings,
        Err(error) => {
            fail(report, source.0, error)?;
            return Ok(ads);
        }
    };
    offerings
        .iter()
        .enumerate()
        .try_fold(ads, |written, (ordinal, offering)| {
            let locator = format!("{}#offering={ordinal}", source.0);
            let rows = nd_sport_coaches(std::slice::from_ref(offering), id, source.0, source.1);
            persist(
                ctx,
                (super::ND_ADAPTER_ID, &locator),
                Table::Coaches,
                &rows,
                report,
            )
            .map(|rows| written.saturating_add(rows))
        })
}
