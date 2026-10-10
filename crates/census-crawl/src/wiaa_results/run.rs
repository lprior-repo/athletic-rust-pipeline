use super::{archive_artifacts, stats_of, Accumulator, Options, Stats, ARCHIVES, PARSE_VERSION};
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::{CanonicalSchool, SchoolId, Sport};
use census_domain::school_index::SchoolIndex;
use census_store::Table;
use std::collections::{HashMap, HashSet};

#[path = "run_artifacts.rs"]
mod run_artifacts;
#[path = "run_receipts.rs"]
mod run_receipts;

use run_artifacts::process_artifact;

pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let mut report = AdapterReport::new("wiaa_results", "artifacts");
    let (requests_before, cache_before) = stats_of(ctx).await;
    let schools = consolidated_schools(ctx)?;
    let mut run = ArtifactRun::new(
        &schools,
        ctx.store.journal_keys(run_receipts::PHASE)?,
        run_receipts::context(ctx, &schools)?,
    );

    for (archive_url, sport) in ARCHIVES {
        if let Err(error) =
            collect_archive(ctx, options, &mut report, &mut run, archive_url, sport).await
        {
            report.errors = report.errors.saturating_add(1);
            report.unfinished.push(format!("{archive_url}: {error}"));
            report.note(format!("{archive_url}: {error}"));
        }
    }

    let counts = append_entities(ctx, run.accumulated, run.pending)?;
    finish_report(
        ctx,
        &mut report,
        &run.stats,
        counts,
        requests_before,
        cache_before,
    )
    .await?;
    report.finish_frontier();
    Ok(report)
}

pub(super) fn consolidated_schools(ctx: &AdapterContext<'_>) -> CrawlResult<Vec<CanonicalSchool>> {
    let mut schools: Vec<CanonicalSchool> = Vec::new();
    ctx.store
        .for_each_merged(Table::Schools, |school: CanonicalSchool| {
            schools.push(school);
            if schools.len() > crate::SCHOOL_BINDINGS {
                return Err(census_store::StoreError::Invariant {
                    detail: "result school-binding resource limit; projection remains unfinished"
                        .into(),
                });
            }
            Ok(())
        })?;
    if schools.is_empty() {
        return Err(CrawlError::Invariant {
            detail: "no consolidated schools: run `collect` and `consolidate` before the \
                     wiaa_results provider"
                .to_string(),
        });
    }
    Ok(schools)
}

async fn finish_report(
    ctx: &AdapterContext<'_>,
    report: &mut AdapterReport,
    stats: &Stats,
    counts: EntityCounts,
    requests_before: u64,
    cache_before: u64,
) -> CrawlResult<()> {
    let (requests_after, cache_after) = stats_of(ctx).await;
    report.rows = u64::try_from(stats.artifacts_parsed).map_err(|_| CrawlError::Arithmetic {
        detail: "parsed artifact count does not fit in u64".to_string(),
    })?;
    report.requests = requests_after.saturating_sub(requests_before);
    report.from_cache = cache_after.saturating_sub(cache_before);
    note_artifacts(report, stats);
    note_entities(report, &counts);
    note_resolution(report, stats);
    Ok(())
}

struct ArtifactRun {
    index: SchoolIndex,
    resolved: HashMap<String, Option<SchoolId>>,
    stats: Stats,
    accumulated: Accumulator,
    visited: HashSet<String>,
    done: HashSet<String>,
    projection_context: String,
    pending: Vec<(String, serde_json::Value)>,
    reported_missing_tool: bool,
}

impl ArtifactRun {
    fn new(schools: &[CanonicalSchool], done: HashSet<String>, projection_context: String) -> Self {
        Self {
            index: SchoolIndex::from_schools(schools),
            resolved: HashMap::new(),
            stats: Stats::default(),
            accumulated: Accumulator::default(),
            visited: HashSet::new(),
            done,
            projection_context,
            pending: Vec::new(),
            reported_missing_tool: false,
        }
    }
}

async fn collect_archive(
    ctx: &AdapterContext<'_>,
    options: &Options,
    report: &mut AdapterReport,
    run: &mut ArtifactRun,
    archive_url: &str,
    sport: Sport,
) -> CrawlResult<()> {
    let archive = ctx.fetcher.get(archive_url, &ctx.fetch_options()).await?;
    let body = archive.text();
    let artifacts = archive_artifacts(&body)?;
    if artifacts.is_empty() {
        report.unfinished.push(format!(
            "{archive_url}: no recognized result artifact catalogue"
        ));
    }
    report.note(format!(
        "{archive_url}: {} result artifacts ({} in the requested seasons)",
        artifacts.len(),
        artifacts
            .iter()
            .filter(
                |artifact| options.seasons.is_empty() || options.seasons.contains(&artifact.year)
            )
            .count()
    ));
    'artifact: for artifact in artifacts {
        if !options.seasons.is_empty() && !options.seasons.contains(&artifact.year) {
            continue;
        }
        if run.visited.contains(&artifact.url) {
            continue;
        }
        if options
            .limit
            .is_some_and(|limit| run.stats.artifacts_seen >= limit)
        {
            report.unfinished.push(format!(
                "{}: artifact limit reached before acquisition",
                artifact.url
            ));
            continue 'artifact;
        }
        run.stats.artifacts_seen = run.stats.artifacts_seen.saturating_add(1);
        process_artifact(ctx, report, run, &artifact, sport).await?;
        run.visited.insert(artifact.url);
    }
    Ok(())
}

struct EntityCounts {
    meets: usize,
    events: usize,
    athletes: usize,
    teams: usize,
    performances: usize,
    unsupported_cohorts: usize,
}

fn append_entities(
    ctx: &AdapterContext<'_>,
    accumulated: Accumulator,
    pending: Vec<(String, serde_json::Value)>,
) -> CrawlResult<EntityCounts> {
    let mut batch = ctx.write_batch();
    accumulated.unsupported.append_to(&mut batch)?;
    batch.commit()?;
    append_rows(
        ctx,
        Table::SourceObservations,
        accumulated.undated.into_iter(),
    )?;
    let counts = EntityCounts {
        meets: append_rows(ctx, Table::Meets, accumulated.meets.into_values())?,
        teams: append_rows(ctx, Table::Teams, accumulated.teams.into_values())?,
        athletes: append_rows(ctx, Table::Athletes, accumulated.athletes.into_values())?,
        events: append_rows(ctx, Table::Events, accumulated.events.into_values())?,
        performances: append_rows(
            ctx,
            Table::Performances,
            accumulated.performances.into_values(),
        )?,
        unsupported_cohorts: accumulated.unsupported.len(),
    };
    let mut batch = ctx.write_batch();
    pending
        .iter()
        .try_for_each(|(key, payload)| batch.journal_done(run_receipts::PHASE, key, payload))?;
    batch.commit()?;
    Ok(counts)
}

fn append_rows<T: serde::Serialize>(
    ctx: &AdapterContext<'_>,
    table: Table,
    mut rows: impl Iterator<Item = T>,
) -> CrawlResult<usize> {
    rows.try_fold(0usize, |count, row| {
        if !ctx.append_row_once(run_receipts::PHASE, table, &row)? {
            return Ok(count);
        }
        count.checked_add(1).ok_or_else(|| CrawlError::Arithmetic {
            detail: "WIAA admitted row count overflow".into(),
        })
    })
}

fn note_artifacts(report: &mut AdapterReport, stats: &Stats) {
    report.note(format!(
        "artifacts: {} seen, {} parsed, {} skipped (unrecognised extension), {} unsupported, {} \
         unparsed-body, {} failed",
        stats.artifacts_seen,
        stats.artifacts_parsed,
        stats.artifacts_unparsed,
        stats.artifacts_unsupported,
        stats.artifacts_parse_failed,
        stats.artifacts_failed
    ));
    report.note(format!(
        "pdf artifacts: {} parsed, {} not a known report layout, {} unreadable (pdftotext)",
        stats.pdf_parsed, stats.pdf_unparsed, stats.pdf_tool_failures
    ));
    report.note(format!("pdf layouts: {:?}", stats.pdf_layouts));
    report.note(format!(
        "formats parsed: {:?}",
        stats
            .formats
            .iter()
            .filter(|(format, _)| format.as_str() != "unparsed")
            .collect::<Vec<_>>()
    ));
    report.note(format!(
        "seasons parsed: {:?}",
        stats.seasons.iter().collect::<Vec<_>>()
    ));
    report.note(format!(
        "result rows: {} (grade-bearing {}), relay legs {}, rows without a school label {}",
        stats.rows, stats.rows_with_grade, stats.relay_legs, stats.rows_without_school
    ));
}

fn note_entities(report: &mut AdapterReport, counts: &EntityCounts) {
    report.note(format!(
        "canonical entities: meets {} events {} athletes {} teams {} performances {}",
        counts.meets, counts.events, counts.athletes, counts.teams, counts.performances
    ));
    report.note(format!(
        "unsupported cohort observations retained: {}",
        counts.unsupported_cohorts
    ));
}

fn note_resolution(report: &mut AdapterReport, stats: &Stats) {
    report.note(format!(
        "school label resolution: {}",
        stats
            .school_resolved
            .iter()
            .map(|(kind, count)| format!("{kind}={count}"))
            .collect::<Vec<_>>()
            .join(" ")
    ));
    if !stats.unresolved.is_empty() {
        let mut unresolved: Vec<(&String, &usize)> = stats.unresolved.iter().collect();
        unresolved.sort_by(|a, b| b.1.cmp(a.1));
        report.note(format!(
            "unresolved school labels ({}): {}",
            stats.unresolved.values().sum::<usize>(),
            unresolved
                .iter()
                .take(12)
                .map(|(name, count)| format!("{name} x{count}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
}
