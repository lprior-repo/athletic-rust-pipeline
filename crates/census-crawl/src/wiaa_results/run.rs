use super::{archive_artifacts, stats_of, Accumulator, Options, Stats, ARCHIVES, PARSE_VERSION};
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam, SchoolId, SourceRef, Sport,
};
use census_domain::school_index::SchoolIndex;
use census_store::Table;
use std::collections::{HashMap, HashSet};

#[path = "run_artifacts.rs"]
mod run_artifacts;

use run_artifacts::process_artifact;

pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let mut report = AdapterReport::new("wiaa_results", "artifacts");
    let (requests_before, cache_before) = stats_of(ctx).await;
    let schools = consolidated_schools(ctx)?;
    let mut run = ArtifactRun::new(&schools, resumed_urls(ctx)?);

    for (archive_url, sport) in ARCHIVES {
        collect_archive(ctx, options, &mut report, &mut run, archive_url, sport).await?;
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
    Ok(report)
}

/// The consolidated schools this walk resolves against. An empty file means `collect` and
/// `consolidate` have not been run yet, which is an operator error rather than a parse failure.
fn consolidated_schools(ctx: &AdapterContext<'_>) -> CrawlResult<Vec<CanonicalSchool>> {
    let schools: Vec<CanonicalSchool> =
        census_store::read::read_rows(&ctx.store.out_dir().join("schools.jsonl"))?;
    if schools.is_empty() {
        return Err(CrawlError::Invariant {
            detail: "no consolidated schools: run `collect` and `consolidate` before the \
                     wiaa_results provider"
                .to_string(),
        });
    }
    Ok(schools)
}

/// Close the report once every archive page has been walked: the row count, the request and cache
/// deltas against the stats captured at entry, and the per-item notes.
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

/// What one walk of the archive pages carries across them: the school index, the resume set, the
/// run counters and the entities minted so far.
struct ArtifactRun {
    index: SchoolIndex,
    source: SourceRef,
    resolved: HashMap<String, Option<SchoolId>>,
    stats: Stats,
    accumulated: Accumulator,
    done: HashSet<String>,
    /// The artifact entries this walk has earned, committed with the entity tables at the end.
    pending: Vec<(String, serde_json::Value)>,
    reported_missing_tool: bool,
}

impl ArtifactRun {
    /// Build the run state from the consolidated schools and the journal's resume set.
    fn new(schools: &[CanonicalSchool], done: HashSet<String>) -> Self {
        Self {
            index: SchoolIndex::from_schools(schools),
            source: SourceRef::new("wiaa_results", None),
            resolved: HashMap::new(),
            stats: Stats::default(),
            accumulated: Accumulator::default(),
            done,
            pending: Vec::new(),
            reported_missing_tool: false,
        }
    }
}

/// The artifact URLs already journaled at the current parser version.
///
/// Only artifacts that yielded entities (or that are deliberately skipped as non-parsable
/// formats) are resumed over; a parse failure is retried on the next run, which is cheap
/// because the body is already in the HTTP cache.
fn resumed_urls(ctx: &AdapterContext<'_>) -> CrawlResult<HashSet<String>> {
    Ok(ctx
        .store
        .journal_payloads("wiaa_results")?
        .into_iter()
        .filter(|entry| {
            entry.get("parser").and_then(serde_json::Value::as_u64)
                == Some(u64::from(PARSE_VERSION))
                && (entry.get("parsed").and_then(serde_json::Value::as_bool) == Some(true)
                    || entry.get("format").and_then(serde_json::Value::as_str)
                        == Some("indexed_only"))
        })
        .filter_map(|entry| {
            entry
                .get("url")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
        .collect())
}

/// Fetch one archive page and read every artifact it lists.
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
        if options
            .limit
            .is_some_and(|limit| run.stats.artifacts_seen >= limit)
        {
            break 'artifact;
        }
        run.stats.artifacts_seen = run.stats.artifacts_seen.saturating_add(1);
        if run.done.contains(&artifact.url) {
            continue;
        }
        process_artifact(ctx, options, report, run, &artifact, sport).await?;
    }
    Ok(())
}

/// Canonical entity counts, for the run note.
struct EntityCounts {
    meets: usize,
    events: usize,
    athletes: usize,
    teams: usize,
    performances: usize,
}

/// Append one batch per table, journal the artifacts this run read, and return what was written.
fn append_entities(
    ctx: &AdapterContext<'_>,
    accumulated: Accumulator,
    pending: Vec<(String, serde_json::Value)>,
) -> CrawlResult<EntityCounts> {
    let meets: Vec<CanonicalMeet> = accumulated.meets.into_values().collect();
    let teams: Vec<CanonicalTeam> = accumulated.teams.into_values().collect();
    let athletes: Vec<CanonicalAthlete> = accumulated.athletes.into_values().collect();
    let events: Vec<CanonicalEvent> = accumulated.events.into_values().collect();
    let performances: Vec<CanonicalPerformance> = accumulated.performances.into_values().collect();
    let mut batch = ctx.write_batch();
    batch.append_many(Table::Meets, &meets)?;
    batch.append_many(Table::Teams, &teams)?;
    batch.append_many(Table::Athletes, &athletes)?;
    batch.append_many(Table::Events, &events)?;
    batch.append_many(Table::Performances, &performances)?;
    for (url, payload) in pending {
        batch.journal_done("wiaa_results", &url, &payload)?;
    }
    batch.commit()?;
    Ok(EntityCounts {
        meets: meets.len(),
        events: events.len(),
        athletes: athletes.len(),
        teams: teams.len(),
        performances: performances.len(),
    })
}

/// Note what the run saw, parsed, skipped and failed on, by format and season.
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

/// Note how many canonical entities the run appended.
fn note_entities(report: &mut AdapterReport, counts: &EntityCounts) {
    report.note(format!(
        "canonical entities: meets {} events {} athletes {} teams {} performances {}",
        counts.meets, counts.events, counts.athletes, counts.teams, counts.performances
    ));
}

/// Note how published school labels resolved, and which ones did not.
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
