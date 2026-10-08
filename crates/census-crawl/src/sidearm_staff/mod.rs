mod discovery;
pub mod map;
pub mod parse;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
pub use discovery::collect_discovered;

use crate::net::{FetchOptions, FetchOutcome};
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::UsJurisdiction;
use census_store::Table;

pub use map::{school_entities, ProfileFacts, SchoolExtract};
pub use parse::{parse_staff_directory, StaffDirectory, StaffRow};

pub const HOST: &str = "https://gomats.org";
pub const DIRECTORY_URL: &str = "https://gomats.org/staff-directory";
pub const SOURCE_ID: &str = "sidearm_staff";
pub const STATE: UsJurisdiction = UsJurisdiction::California;
const PHASE: &str = "sidearm_staff_projection_v2";

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: String,
    pub states: Vec<UsJurisdiction>,
    pub school_names: Vec<String>,
}

#[tracing::instrument(skip(ctx, options))]
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let mut report = AdapterReport::new(SOURCE_ID, "schools");
    if !options.states.is_empty() && !options.states.contains(&STATE) {
        report.note("this adapter covers only the verified gomats.org school in CA");
        return Ok(report);
    }
    if options.limit == Some(0) {
        report.unfinished.push(DIRECTORY_URL.to_string());
        report.disposition = crate::CollectionDisposition::Partial;
        return Ok(report);
    }
    let before = ctx.fetcher.stats().await;
    refresh_school(ctx, options, &mut report).await?;
    let after = ctx.fetcher.stats().await;
    report.requests = after
        .physical_requests()
        .checked_sub(before.physical_requests())
        .ok_or_else(counter_error)?;
    report.from_cache = after
        .cache_hits
        .checked_sub(before.cache_hits)
        .ok_or_else(counter_error)?;
    report.finish_frontier();
    Ok(report)
}

fn counter_error() -> CrawlError {
    CrawlError::Arithmetic {
        detail: "SIDEARM counter overflow or reversal".to_string(),
    }
}

#[tracing::instrument(skip(ctx, options, report))]
async fn refresh_school(
    ctx: &AdapterContext<'_>,
    options: &Options,
    report: &mut AdapterReport,
) -> CrawlResult<()> {
    let fetch = FetchOptions {
        refresh: ctx.refresh || options.refresh,
        ..ctx.fetch_options()
    };
    let outcome = match ctx.fetcher.get(DIRECTORY_URL, &fetch).await {
        Ok(outcome) if outcome.status == 200 => outcome,
        Ok(outcome) => {
            fail(report, format!("HTTP {}", outcome.status));
            return Ok(());
        }
        Err(error) => {
            fail(report, error.to_string());
            return Ok(());
        }
    };
    let directory = match std::str::from_utf8(&outcome.body)
        .map_err(|error| error.to_string())
        .and_then(|text| parse_staff_directory(text).map_err(|error| error.to_string()))
    {
        Ok(directory) => directory,
        Err(error) => {
            fail(report, error);
            return Ok(());
        }
    };
    project(ctx, options, &outcome, &directory, report)
}

fn project(
    ctx: &AdapterContext<'_>,
    options: &Options,
    capture: &FetchOutcome,
    directory: &StaffDirectory,
    report: &mut AdapterReport,
) -> CrawlResult<()> {
    if !options.school_names.is_empty()
        && !options
            .school_names
            .iter()
            .any(|name| directory.name.contains(name))
    {
        fail(
            report,
            "requested school does not match qualified owner".to_string(),
        );
        return Ok(());
    }
    let mut extract = school_entities(
        &ProfileFacts {
            state: STATE,
            host: HOST,
            url: &capture.url,
            observed_on: &capture.fetched_at,
        },
        directory,
    );
    capture_provenance(&mut extract, capture);
    if !extract.coaches.is_empty() {
        report.unfinished.push(capture.url.clone());
        report.note("appointment season is not published; contact tenure remains unknown");
    }
    emit_school(ctx, &extract, capture)?;
    crate::coach_directories::persist_staff_capture(
        ctx,
        &extract.school,
        capture,
        &extract.coaches,
        census_domain::model::ContactResearchOutcome::CompletedEmpty,
    )?;
    report.rows = 1;
    report.with_email = u64::try_from(
        extract
            .coaches
            .iter()
            .filter(|coach| coach.has_published_email())
            .count(),
    )
    .map_err(|_| counter_error())?;
    Ok(())
}

fn fail(report: &mut AdapterReport, detail: String) {
    report.errors = 1;
    report.unfinished.push(DIRECTORY_URL.to_string());
    report.note(detail.chars().take(4096).collect::<String>());
}

fn emit_school(
    ctx: &AdapterContext<'_>,
    extract: &SchoolExtract,
    capture: &FetchOutcome,
) -> CrawlResult<()> {
    let key = census_domain::model::Id::<()>::mint(
        PHASE,
        &[
            extract.school.id.as_str(),
            &capture.url,
            &capture.content_digest,
            &capture.fetched_at,
        ],
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
            &map::namespace(extract.state),
            std::slice::from_ref(&extract.school),
            &capture.fetched_at,
        ),
    )?;
    batch.append_many(Table::Coaches, &extract.coaches)?;
    batch.journal_done(PHASE, &key, &serde_json::json!({"capture_url":capture.url,"sha256":capture.content_digest,"fetched_at":capture.fetched_at}))?;
    batch.commit_once(&operation, &capture.content_digest)?;
    Ok(())
}

fn capture_provenance(extract: &mut SchoolExtract, capture: &FetchOutcome) {
    let provenance = serde_json::json!({"capture_url":capture.url,"sha256":capture.content_digest,"fetched_at":capture.fetched_at}).to_string();
    extract
        .school
        .evidence
        .iter_mut()
        .for_each(|evidence| evidence.note = Some(provenance.clone()));
    extract.coaches.iter_mut().for_each(|coach| {
        coach.evidence.iter_mut().for_each(|evidence| {
            evidence.note = Some(format!(
                "{}; {provenance}",
                evidence.note.as_deref().map_or("", |value| value)
            ));
        })
    });
}
