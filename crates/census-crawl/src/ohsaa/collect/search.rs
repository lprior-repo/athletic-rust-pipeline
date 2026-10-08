use super::super::map::SearchResult;
use super::super::parse::resolve_school_name;
use super::super::{Options, ASSOCIATION, HOST, SEARCH_PATH};
use super::{counter_error, fail, owe};
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::{CanonicalSchool, SourceNamespace};
use census_store::Table;
use futures::{stream, StreamExt, TryStreamExt};

const MAX_SCHOOLS: usize = 2048;
const MAX_SEARCH_BYTES: usize = 1024 * 1024;

pub(super) async fn resolve_schools(
    ctx: &AdapterContext<'_>,
    options: &Options,
    report: &mut AdapterReport,
) -> CrawlResult<Vec<SearchResult>> {
    let mut rows = if options.school_names.is_empty() {
        existing_schools(ctx, report)?
    } else {
        search_schools(ctx, options, report).await?
    };
    if rows.is_empty() {
        owe(report, &format!("{HOST}{SEARCH_PATH}"))?;
    }
    if let Some(limit) = options.limit {
        rows.iter().skip(limit).try_for_each(|row| {
            owe(report, &row.sports_url())?;
            owe(report, &row.ad_url())
        })?;
        rows.truncate(limit);
    }
    Ok(rows)
}

fn existing_schools(
    ctx: &AdapterContext<'_>,
    report: &mut AdapterReport,
) -> CrawlResult<Vec<SearchResult>> {
    let mut rows = Vec::new();
    let mut failure = None;
    ctx.store
        .for_each_merged(Table::Schools, |school: CanonicalSchool| {
            if failure.is_none() {
                if let Err(error) = existing_school(school, report, &mut rows) {
                    failure = Some(error);
                }
            }
            Ok(())
        })?;
    match failure {
        Some(error) => Err(error),
        None => Ok(rows),
    }
}

fn existing_school(
    school: CanonicalSchool,
    report: &mut AdapterReport,
    rows: &mut Vec<SearchResult>,
) -> CrawlResult<()> {
    if school.association.as_deref() != Some(ASSOCIATION) {
        return Ok(());
    }
    let namespace = SourceNamespace::association_school(ASSOCIATION);
    let owner = school.source_identities.iter().find(|identity| {
        identity.namespace == namespace
            && !identity.id.is_empty()
            && identity.id.bytes().all(|byte| byte.is_ascii_digit())
    });
    let Some(owner) = owner else {
        return fail(
            report,
            &format!("ohsaa:school:{}", school.id),
            "indexed school has no usable OHSAA owner",
        );
    };
    let row = SearchResult {
        ohsaa_id: owner.id.clone(),
        name: school.name,
        city: school
            .city
            .map_or_else(String::new, core::convert::identity),
    };
    push_school(rows, row, report)
}

async fn search_schools(
    ctx: &AdapterContext<'_>,
    options: &Options,
    report: &mut AdapterReport,
) -> CrawlResult<Vec<SearchResult>> {
    let (_, rows) = stream::iter(&options.school_names)
        .map(Ok::<_, CrawlError>)
        .try_fold(
            (report, Vec::new()),
            |(report, mut rows), name| async move {
                let url = format!("{HOST}{SEARCH_PATH}?Name={}", url_encode(name)?);
                match ctx.fetcher.get(&url, &ctx.fetch_options()).await {
                    Ok(capture) if capture.status == 200 => {
                        project_search(&capture, name, &url, &mut rows, report)?
                    }
                    Ok(capture) => fail(
                        report,
                        &url,
                        &format!("school search returned HTTP {}", capture.status),
                    )?,
                    Err(error) => fail(report, &url, &error.to_string())?,
                }
                Ok((report, rows))
            },
        )
        .await?;
    Ok(rows)
}

fn project_search(
    capture: &crate::net::FetchOutcome,
    name: &str,
    url: &str,
    rows: &mut Vec<SearchResult>,
    report: &mut AdapterReport,
) -> CrawlResult<()> {
    if capture.body.len() > MAX_SEARCH_BYTES {
        return fail(report, url, "school search exceeds bounded capture budget");
    }
    let html = match std::str::from_utf8(&capture.body) {
        Ok(html) => html,
        Err(error) => return fail(report, url, &error.to_string()),
    };
    let mut notes = Vec::new();
    match resolve_school_name(html, name, &mut notes) {
        Some(row) => push_school(rows, row, report)?,
        None => fail(
            report,
            url,
            "requested school has no unambiguous published owner",
        )?,
    }
    notes.into_iter().take(5).for_each(|note| {
        if report.notes.len() < 5 {
            report.note(note.chars().take(4096).collect::<String>());
        }
    });
    Ok(())
}

fn push_school(
    rows: &mut Vec<SearchResult>,
    row: SearchResult,
    report: &mut AdapterReport,
) -> CrawlResult<()> {
    if row.name.len() > 4096 || row.city.len() > 4096 || row.ohsaa_id.len() > 4096 {
        return fail(
            report,
            &row.sports_url(),
            "resolved school fields exceed source budget",
        );
    }
    if rows.len() >= MAX_SCHOOLS {
        fail(
            report,
            &row.sports_url(),
            "school frontier capacity reached",
        )?;
        return owe(report, &row.ad_url());
    }
    rows.try_reserve(1).map_err(|_| CrawlError::Resource {
        resource: "Ohio resolved schools",
        requested: 1,
        limit: MAX_SCHOOLS,
    })?;
    rows.push(row);
    Ok(())
}

fn url_encode(value: &str) -> CrawlResult<String> {
    url::form_urlencoded::byte_serialize(value.as_bytes()).try_fold(
        String::new(),
        |mut encoded, part| {
            let part = if part == "+" { "%20" } else { part };
            let size = encoded
                .len()
                .checked_add(part.len())
                .ok_or_else(counter_error)?;
            encoded
                .try_reserve(part.len())
                .map_err(|_| CrawlError::Resource {
                    resource: "Ohio search URL",
                    requested: size,
                    limit: value.len().saturating_mul(3),
                })?;
            encoded.push_str(part);
            Ok(encoded)
        },
    )
}
