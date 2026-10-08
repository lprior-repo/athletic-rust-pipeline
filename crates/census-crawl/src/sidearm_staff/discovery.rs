use super::{Options, SOURCE_ID};
use crate::net::{FetchOptions, FetchOutcome};
use crate::{AdapterContext, AdapterReport, CollectionDisposition, CrawlError, CrawlResult};
use census_domain::model::{normalize_name, CanonicalSchool, ContactResearchOutcome as Outcome};
use std::collections::BTreeSet;

mod projection;

use projection::{capture_attempt, project};

#[tracing::instrument(skip(ctx, options, schools))]
pub async fn collect_discovered(
    ctx: &AdapterContext<'_>,
    options: &Options,
    schools: &[CanonicalSchool],
) -> CrawlResult<AdapterReport> {
    if schools.len() > 64 {
        return Err(CrawlError::Resource {
            resource: "SIDEARM discovery schools",
            requested: schools.len(),
            limit: 64,
        });
    }
    let before = ctx.fetcher.stats().await;
    let mut report = AdapterReport::new(SOURCE_ID, "school_sites");
    for (index, school) in schools
        .iter()
        .filter(|school| requested(options, school))
        .enumerate()
    {
        if options.limit.is_some_and(|limit| index >= limit) {
            pending(ctx, school, &school.id.to_string(), &mut report)?;
            continue;
        }
        discover_school(ctx, options, school, &mut report).await?;
    }
    let after = ctx.fetcher.stats().await;
    report.requests = after
        .requests
        .checked_sub(before.requests)
        .ok_or_else(super::counter_error)?;
    report.from_cache = after
        .cache_hits
        .checked_sub(before.cache_hits)
        .ok_or_else(super::counter_error)?;
    if !matches!(
        report.disposition,
        CollectionDisposition::Blocked | CollectionDisposition::Failed
    ) {
        report.finish_frontier();
    }
    Ok(report)
}

fn requested(options: &Options, school: &CanonicalSchool) -> bool {
    (options.states.is_empty()
        || school
            .state
            .is_some_and(|state| options.states.contains(&state)))
        && (options.school_names.is_empty()
            || options
                .school_names
                .iter()
                .any(|name| normalize_name(name) == school.normalized_name))
}

async fn discover_school(
    ctx: &AdapterContext<'_>,
    options: &Options,
    school: &CanonicalSchool,
    report: &mut AdapterReport,
) -> CrawlResult<()> {
    let seeds: BTreeSet<_> = school
        .athletics_website
        .iter()
        .chain(&school.school_website)
        .map(String::as_str)
        .collect();
    if seeds.is_empty() {
        return pending(ctx, school, &school.id.to_string(), report);
    }
    for seed in seeds {
        let Some(capture) = acquire(ctx, options, seed, report).await? else {
            pending(ctx, school, seed, report)?;
            continue;
        };
        let inspection = crate::coach_directories::inspect_staff_links(&capture.body);
        let (qualified, links) = match inspection {
            Ok(inspection) => inspection,
            Err(error) => {
                failed_capture(ctx, school, &capture, error.to_string(), report)?;
                continue;
            }
        };
        if qualified {
            project(ctx, school, &capture, report)?;
            continue;
        }
        let routes = published_routes(&capture, seed, &links)?;
        if routes.is_empty() {
            pending(ctx, school, seed, report)?;
            continue;
        }
        for (index, route) in routes.iter().enumerate() {
            if index >= 6 {
                pending(ctx, school, route, report)?;
                continue;
            }
            let Some(page) = acquire(ctx, options, route, report).await? else {
                pending(ctx, school, route, report)?;
                continue;
            };
            match crate::coach_directories::inspect_staff_links(&page.body) {
                Ok((true, _)) => project(ctx, school, &page, report)?,
                Ok((false, _)) => pending(ctx, school, route, report)?,
                Err(error) => failed_capture(ctx, school, &page, error.to_string(), report)?,
            }
        }
    }
    Ok(())
}

fn published_routes(
    capture: &FetchOutcome,
    seed: &str,
    links: &[String],
) -> CrawlResult<BTreeSet<String>> {
    let authorized = url::Url::parse(seed).map_err(|error| schema(seed, error.to_string()))?;
    let base =
        url::Url::parse(locator(capture)).map_err(|error| schema(seed, error.to_string()))?;
    if base.origin() != authorized.origin() {
        return Err(schema(
            seed,
            "discovery capture left its source-discovered origin".to_owned(),
        ));
    }
    let mut routes = BTreeSet::new();
    for href in links {
        if !["staff", "directory"].iter().any(|word| {
            href.as_bytes()
                .windows(word.len())
                .any(|part| part.eq_ignore_ascii_case(word.as_bytes()))
        }) {
            continue;
        }
        let route = base
            .join(href)
            .map_err(|error| schema(seed, error.to_string()))?;
        if matches!(route.scheme(), "http" | "https")
            && route.origin() == authorized.origin()
            && route != base
        {
            routes.insert(route.to_string());
        }
    }
    Ok(routes)
}

async fn acquire(
    ctx: &AdapterContext<'_>,
    options: &Options,
    url: &str,
    report: &mut AdapterReport,
) -> CrawlResult<Option<FetchOutcome>> {
    let fetch = FetchOptions {
        refresh: ctx.refresh || options.refresh,
        ..ctx.fetch_options()
    };
    match ctx.fetcher.get(url, &fetch).await {
        Ok(capture) if capture.status == 200 => {
            if capture.body.len() > 1024 * 1024 {
                failure(
                    report,
                    url,
                    "staff discovery page exceeds 1 MiB".to_owned(),
                    Outcome::Partial,
                )?;
                return Ok(None);
            }
            if let Err(error) = published_routes(&capture, url, &[]) {
                failure(report, url, error.to_string(), Outcome::Ambiguous)?;
                return Ok(None);
            }
            if crate::net::cache::content_digest(&capture.body) != capture.content_digest {
                failure(
                    report,
                    url,
                    "discovery capture digest differs from retained bytes".to_owned(),
                    Outcome::Failed,
                )?;
                return Ok(None);
            }
            Ok(Some(capture))
        }
        Ok(capture) => {
            failure(
                report,
                url,
                format!("HTTP {}", capture.status),
                crate::coach_directories::contact_status_failure(capture.status),
            )?;
            Ok(None)
        }
        Err(error) => {
            failure(
                report,
                url,
                error.to_string(),
                crate::coach_directories::contact_fetch_failure(&error),
            )?;
            Ok(None)
        }
    }
}

fn pending(
    ctx: &AdapterContext<'_>,
    school: &CanonicalSchool,
    url: &str,
    report: &mut AdapterReport,
) -> CrawlResult<()> {
    crate::coach_directories::persist_unattempted(ctx, school)?;
    owe(report, url);
    Ok(())
}

fn failed_capture(
    ctx: &AdapterContext<'_>,
    school: &CanonicalSchool,
    capture: &FetchOutcome,
    reason: String,
    report: &mut AdapterReport,
) -> CrawlResult<()> {
    crate::coach_directories::persist_staff_attempt(
        ctx,
        school,
        capture_attempt(capture, Outcome::Failed, reason.clone()),
    )?;
    failure(report, locator(capture), reason, Outcome::Failed)
}

fn failure(
    report: &mut AdapterReport,
    url: &str,
    reason: String,
    outcome: Outcome,
) -> CrawlResult<()> {
    report.errors = report
        .errors
        .checked_add(1)
        .ok_or_else(super::counter_error)?;
    owe(report, url);
    if outcome == Outcome::Blocked {
        report.disposition = CollectionDisposition::Blocked;
    } else if report.disposition != CollectionDisposition::Blocked {
        report.disposition = if outcome == Outcome::Failed {
            CollectionDisposition::Failed
        } else {
            CollectionDisposition::Partial
        };
    }
    if report.errors <= 5 {
        report.note(reason.chars().take(4096).collect::<String>());
    }
    Ok(())
}

fn owe(report: &mut AdapterReport, url: &str) {
    if !report.unfinished.iter().any(|owed| owed == url) {
        report.unfinished.push(url.to_owned());
    }
}

fn locator(capture: &FetchOutcome) -> &str {
    capture
        .response_url
        .as_deref()
        .map_or(capture.url.as_str(), |url| url)
}

fn schema(url: &str, detail: String) -> CrawlError {
    CrawlError::Schema {
        url: url.to_owned(),
        detail,
    }
}
