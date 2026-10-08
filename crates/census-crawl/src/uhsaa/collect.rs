use super::{parse, school_entities, Options, ProfileFacts, SchoolExtract, ASSOCIATION, HOST};
use crate::net::{FetchOptions, FetchOutcome};
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::UsJurisdiction;
use census_store::Table;
use futures::{stream, StreamExt, TryStreamExt};

const DIRECTORY: &str = "https://uhsaa.org/school-directory-new/";
const PHASE: &str = "uhsaa_projection_v2";

#[tracing::instrument(skip(ctx, options))]
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let report = AdapterReport::new("uhsaa", "schools");
    if !options.states.is_empty() && !options.states.contains(&UsJurisdiction::Utah) {
        return Ok(report);
    }
    let before = ctx.fetcher.stats().await;
    let fetch = FetchOptions {
        refresh: ctx.refresh || options.refresh,
        ..ctx.fetch_options()
    };
    let mut report = walk(ctx, options, &fetch, report).await?;
    let after = ctx.fetcher.stats().await;
    report.requests = after
        .physical_requests()
        .checked_sub(before.physical_requests())
        .ok_or_else(counter_error)?;
    report.from_cache = after
        .cache_hits
        .checked_sub(before.cache_hits)
        .ok_or_else(counter_error)?;
    Ok(report)
}

fn counter_error() -> CrawlError {
    CrawlError::Arithmetic {
        detail: "UHSAA counter overflow or reversal".to_string(),
    }
}

#[tracing::instrument(skip(ctx, options, fetch, report))]
async fn walk(
    ctx: &AdapterContext<'_>,
    options: &Options,
    fetch: &FetchOptions,
    mut report: AdapterReport,
) -> CrawlResult<AdapterReport> {
    let Some(capture) = get(ctx, DIRECTORY, fetch, &mut report).await? else {
        return Ok(report);
    };
    let html = match std::str::from_utf8(&capture.body) {
        Ok(html) => html,
        Err(error) => {
            fail(&mut report, DIRECTORY, error.to_string())?;
            return Ok(report);
        }
    };
    let links = parse::parse_directory_links(html);
    if links.is_empty() {
        fail(
            &mut report,
            DIRECTORY,
            "no validated school links".to_string(),
        )?;
        return Ok(report);
    }
    options.school_names.iter().try_for_each(|name| {
        if links.iter().any(|link| link.name.contains(name)) {
            Ok(())
        } else {
            fail(
                &mut report,
                DIRECTORY,
                format!("requested school {name:?} not indexed"),
            )
        }
    })?;
    let mut report = stream::iter(
        links
            .iter()
            .filter(|link| {
                options.school_names.is_empty()
                    || options
                        .school_names
                        .iter()
                        .any(|name| link.name.contains(name))
            })
            .enumerate(),
    )
    .map(Ok::<_, CrawlError>)
    .try_fold(report, |mut report, (index, link)| async move {
        let url = profile_url(link);
        if options.limit.is_some_and(|limit| index >= limit) {
            owe(&mut report, &url)?;
        } else {
            refresh_school(ctx, link, fetch, &mut report).await?;
        }
        Ok(report)
    })
    .await?;
    report.finish_frontier();
    Ok(report)
}

#[tracing::instrument(skip(ctx, link, fetch, report))]
async fn refresh_school(
    ctx: &AdapterContext<'_>,
    link: &parse::SchoolLink,
    fetch: &FetchOptions,
    report: &mut AdapterReport,
) -> CrawlResult<()> {
    let url = profile_url(link);
    let Some(capture) = get(ctx, &url, fetch, report).await? else {
        return Ok(());
    };
    let html = match std::str::from_utf8(&capture.body) {
        Ok(html) => html,
        Err(error) => {
            fail(report, &url, error.to_string())?;
            return Ok(());
        }
    };
    let profile = parse::parse_school_profile(html);
    if profile.name.is_empty()
        || census_domain::model::normalize_name(&profile.name)
            != census_domain::model::normalize_name(&link.name)
    {
        fail(
            report,
            &url,
            "profile has no matching published school owner".to_string(),
        )?;
        return Ok(());
    }
    let extract = school_entities(
        &ProfileFacts {
            name: &profile.name,
            address: &profile.address,
            district: &profile.district,
            classification: &profile.classification,
            region: &profile.region,
            url: &capture.url,
            observed_on: &capture.fetched_at,
        },
        &profile.coaches,
    );
    if !extract.coaches.is_empty() {
        owe(report, &url)?;
        report.note("appointment season is not published; tenure remains unknown");
    }
    emit_school(ctx, &extract, &capture)?;
    crate::coach_directories::persist_staff_capture(
        ctx,
        &extract.school,
        &capture,
        &extract.coaches,
        census_domain::model::ContactResearchOutcome::CompletedEmpty,
    )?;
    report.rows = report.rows.checked_add(1).ok_or_else(counter_error)?;
    report.with_email = report
        .with_email
        .checked_add(
            u64::try_from(
                extract
                    .coaches
                    .iter()
                    .filter(|coach| coach.has_published_email())
                    .count(),
            )
            .map_err(|_| counter_error())?,
        )
        .ok_or_else(counter_error)?;
    Ok(())
}

fn profile_url(link: &parse::SchoolLink) -> String {
    if link.url.starts_with("http") {
        return link.url.clone();
    }
    format!(
        "{HOST}/{}",
        link.url
            .strip_prefix("../")
            .map_or(link.url.as_str(), |value| value)
    )
}

#[tracing::instrument(skip(ctx, fetch, report))]
async fn get(
    ctx: &AdapterContext<'_>,
    url: &str,
    fetch: &FetchOptions,
    report: &mut AdapterReport,
) -> CrawlResult<Option<FetchOutcome>> {
    match ctx.fetcher.get(url, fetch).await {
        Ok(capture) if capture.status == 200 => Ok(Some(capture)),
        Ok(capture) => {
            fail(report, url, format!("HTTP {}", capture.status))?;
            Ok(None)
        }
        Err(error) => {
            fail(report, url, error.to_string())?;
            Ok(None)
        }
    }
}

fn owe(report: &mut AdapterReport, url: &str) -> CrawlResult<()> {
    if report.unfinished.iter().any(|value| value == url) {
        return Ok(());
    }
    report
        .unfinished
        .try_reserve(1)
        .map_err(|_| CrawlError::Resource {
            resource: "UHSAA locators",
            requested: 1,
            limit: crate::net::MAX_BODY_BYTES,
        })?;
    report.unfinished.push(url.to_string());
    report.disposition = crate::CollectionDisposition::Partial;
    Ok(())
}

fn fail(report: &mut AdapterReport, url: &str, detail: String) -> CrawlResult<()> {
    report.errors = report.errors.checked_add(1).ok_or_else(counter_error)?;
    owe(report, url)?;
    if report.errors <= 5 {
        report.note(
            format!("{url}: {detail}")
                .chars()
                .take(4096)
                .collect::<String>(),
        );
    }
    Ok(())
}

fn emit_school(
    ctx: &AdapterContext<'_>,
    extract: &SchoolExtract,
    capture: &FetchOutcome,
) -> CrawlResult<()> {
    let key = census_domain::model::Id::<()>::mint(
        PHASE,
        &[&capture.url, &capture.content_digest, &capture.fetched_at],
    )
    .to_string();
    let operation = format!("{PHASE}:{key}");
    if ctx.effect_is_committed(&operation, &capture.content_digest)? {
        return Ok(());
    }
    let mut batch = ctx.write_batch();
    batch.append_many(Table::Schools, std::slice::from_ref(&extract.school))?;
    batch.append_many(
        Table::SourceObservations,
        &crate::school_observations_of(
            &census_domain::model::SourceNamespace::association_school(ASSOCIATION),
            std::slice::from_ref(&extract.school),
            &capture.fetched_at,
        ),
    )?;
    batch.append_many(Table::Coaches, &extract.coaches)?;
    batch.journal_done(PHASE, &key, &serde_json::json!({"url":capture.url,"sha256":capture.content_digest,"fetched_at":capture.fetched_at}))?;
    batch.commit_once(&operation, &capture.content_digest)?;
    Ok(())
}
