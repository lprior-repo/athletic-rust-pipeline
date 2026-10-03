mod write;

use super::map::{capture_note, school_entities};
use super::pages::checked::parse_capture;
use super::{Options, ASSOCIATION};
use crate::net::{FetchError, FetchOptions, FetchOutcome};
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::{SourceNamespace, SourceObservation, SourceSchoolObservation};

const JOURNAL: &str = "riil_schools";

#[tracing::instrument(skip(ctx, options))]
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    collect_directory(ctx, options, &format!("{}/Directory.aspx", super::HOST)).await
}

#[tracing::instrument(skip(ctx, options))]
pub(super) async fn collect_directory(
    ctx: &AdapterContext<'_>,
    options: &Options,
    url: &str,
) -> CrawlResult<AdapterReport> {
    let mut report = AdapterReport::new("riil", "schools");
    let before = ctx.fetcher.stats().await;
    let fetched = ctx
        .fetcher
        .get(
            url,
            &FetchOptions {
                refresh: options.refresh || ctx.refresh,
                ..ctx.fetch_options()
            },
        )
        .await;
    let mut tally = Tally::default();
    match fetched {
        Ok(capture) => match apply_directory(ctx, &capture, &mut report, &mut tally) {
            Ok(()) => report.note(format!(
                "parsed {} schools with {} XC/TF coach rows; {} identical committed schools skipped",
                tally.schools, tally.coaches, tally.skipped
            )),
            Err(CrawlError::Store(error)) => return Err(error.into()),
            Err(error) => fail(&mut report, error.to_string()),
        },
        Err(error) => fail(&mut report, format!("directory {url}: {error}")),
    }
    let after = ctx.fetcher.stats().await;
    report.requests = after.requests.saturating_sub(before.requests);
    report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
    Ok(report)
}

fn fail(report: &mut AdapterReport, detail: String) {
    report.errors = report.errors.saturating_add(1);
    report.note(format!("unfinished RIIL directory: {detail}"));
}

fn apply_directory(
    ctx: &AdapterContext<'_>,
    capture: &FetchOutcome,
    report: &mut AdapterReport,
    tally: &mut Tally,
) -> CrawlResult<()> {
    if capture.status != 200 {
        return Err(FetchError::Http {
            status: capture.status,
            url: capture.url.clone(),
        }
        .into());
    }
    let tables = parse_capture(capture)?;
    if tables.is_empty() {
        report.note("published directory contains no school entries");
    }
    tables
        .iter()
        .try_for_each(|table| process_school(ctx, table, capture, report, tally))
}

#[derive(Default)]
struct Tally {
    schools: usize,
    coaches: usize,
    skipped: usize,
}

fn process_school(
    ctx: &AdapterContext<'_>,
    table: &super::map::SchoolTable,
    capture: &FetchOutcome,
    report: &mut AdapterReport,
    tally: &mut Tally,
) -> CrawlResult<()> {
    let extract = school_entities(table, capture);
    tally.schools = tally.schools.saturating_add(1);
    tally.coaches = tally.coaches.saturating_add(extract.coaches.len());
    let school_key = format!("RI:{}:{}", extract.school.id, capture.content_digest);
    if ctx.store.journal_contains(JOURNAL, &school_key)? {
        tally.skipped = tally.skipped.saturating_add(1);
        return Ok(());
    }
    let observation = SourceSchoolObservation::of_school(
        &SourceNamespace::association_school(ASSOCIATION),
        &extract.school,
        &capture.fetched_at,
    )
    .map(|mut observation| {
        observation.source_row_key = capture_note(capture).to_string();
        SourceObservation::School(observation)
    });
    let payload = serde_json::json!({
        "name": extract.school.name,
        "coaches": extract.coaches.len(),
        "xc_tf_coaches": extract.coaches.len(),
        "capture": capture_note(capture),
    });
    if write::persist(
        ctx,
        &extract,
        observation.as_slice(),
        &school_key,
        capture,
        &payload,
    )? {
        report.rows = report.rows.saturating_add(1);
    } else {
        tally.skipped = tally.skipped.saturating_add(1);
    }
    Ok(())
}
