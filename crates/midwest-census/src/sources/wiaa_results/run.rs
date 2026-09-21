use super::map::absorb;
use super::parse::{parse_pdf, pdftotext};
use super::{
    archive_artifacts, artifact_format, stats_of, Accumulator, ArtifactFormat, Options, Stats,
    ARCHIVES, PARSE_VERSION,
};
use crate::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalTeam, SchoolId,
    SourceRef,
};
use crate::school_index::SchoolIndex;
use crate::sources::{AdapterContext, AdapterReport};
use crate::store::Table;
use anyhow::{Context, Result};
use serde_json::json;
use std::collections::HashMap;

pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> Result<AdapterReport> {
    let mut report = AdapterReport::new("wiaa_results", "artifacts");
    let (requests_before, cache_before) = stats_of(ctx).await;

    let schools: Vec<crate::model::CanonicalSchool> =
        crate::report::read_rows(&ctx.store.out_dir().join("schools.jsonl"))?;
    anyhow::ensure!(
        !schools.is_empty(),
        "no consolidated schools: run `collect` and `consolidate` before the wiaa_results provider"
    );
    let index = SchoolIndex::from_schools(&schools);
    let mut resolved: HashMap<String, Option<SchoolId>> = HashMap::new();

    let mut stats = Stats::default();
    let mut accumulated = Accumulator::default();
    let source = SourceRef::new("wiaa_results", None);
    // Only artifacts that yielded entities (or that are deliberately skipped as non-parsable
    // formats) are resumed over; a parse failure is retried on the next run, which is cheap
    // because the body is already in the HTTP cache.
    let mut reported_missing_tool = false;
    let done: std::collections::HashSet<String> = ctx
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
        .collect();

    for (archive_url, sport) in ARCHIVES {
        let archive = ctx
            .fetcher
            .get(archive_url, &ctx.fetch_options())
            .await
            .with_context(|| format!("fetching the WIAA archive page {archive_url}"))?;
        let body = archive.text();
        let artifacts = archive_artifacts(&body)?;
        report.note(format!(
            "{archive_url}: {} result artifacts ({} in the requested seasons)",
            artifacts.len(),
            artifacts
                .iter()
                .filter(|artifact| options.seasons.is_empty()
                    || options.seasons.contains(&artifact.year))
                .count()
        ));
        'artifact: for artifact in artifacts {
            if !options.seasons.is_empty() && !options.seasons.contains(&artifact.year) {
                continue;
            }
            if options
                .limit
                .is_some_and(|limit| stats.artifacts_seen >= limit)
            {
                break 'artifact;
            }
            stats.artifacts_seen = stats.artifacts_seen.saturating_add(1);
            if done.contains(&artifact.url) {
                continue;
            }
            let extension = artifact.extension.as_str();
            if artifact_format(extension, None) == ArtifactFormat::Unparsed {
                stats.artifacts_unparsed = stats.artifacts_unparsed.saturating_add(1);
                let formats = stats.formats.entry(extension.to_string()).or_default();
                *formats = formats.saturating_add(1);
                ctx.store.journal_done(
                    "wiaa_results",
                    &artifact.url,
                    &json!({
                        "url": artifact.url,
                        "parser": PARSE_VERSION,
                        "format": "indexed_only",
                        "year": artifact.year,
                        "stem": artifact.stem
                    }),
                )?;
                continue;
            }
            let fetched = match ctx.fetcher.get(&artifact.url, &ctx.fetch_options()).await {
                Ok(meta) => meta,
                Err(error) => {
                    stats.artifacts_failed = stats.artifacts_failed.saturating_add(1);
                    report.note(format!("{}: {error}", artifact.url));
                    continue;
                }
            };
            let body = fetched.text();
            let format = artifact_format(extension, Some(&body));
            if format == ArtifactFormat::Unparsed {
                stats.artifacts_unsupported = stats.artifacts_unsupported.saturating_add(1);
                report.note(format!("{}: unrecognised result format", artifact.url));
                continue;
            }
            let parsed = match format {
                ArtifactFormat::HytekHtml => crate::sources::hytek::parse(
                    &crate::sources::hytek::lines_from_html(&body),
                    source.clone(),
                ),
                ArtifactFormat::HytekText => crate::sources::hytek::parse(
                    &crate::sources::hytek::lines_from_text(&body),
                    source.clone(),
                ),
                ArtifactFormat::RaceDay => {
                    // RaceDay reports its own typed parse error; for this runner a body that is not a
                    // RaceDay report is the same thing the other arms return as `None` — an artifact
                    // that yielded no meet, reported below as an unparsed body. The cause goes to the
                    // run notes so the failure is diagnosable rather than only counted.
                    match crate::sources::raceday::parse(&body, source.clone(), artifact.year) {
                        Ok(parsed) => Some(parsed),
                        Err(error) => {
                            report.note(format!("{}: {error}", artifact.url));
                            None
                        }
                    }
                }
                ArtifactFormat::Pdf => match pdftotext(&fetched.body) {
                    Ok(text) => {
                        let (parsed, layout) = parse_pdf(&text, source.clone(), artifact.year);
                        if let Some(layout) = layout {
                            stats.pdf_parsed = stats.pdf_parsed.saturating_add(1);
                            let layouts = stats.pdf_layouts.entry(layout.to_string()).or_default();
                            *layouts = layouts.saturating_add(1);
                        } else {
                            stats.pdf_unparsed = stats.pdf_unparsed.saturating_add(1);
                            report.note(format!(
                                "{}: pdf text is not a report this parser knows",
                                artifact.url
                            ));
                        }
                        parsed
                    }
                    Err(error) => {
                        stats.pdf_tool_failures = stats.pdf_tool_failures.saturating_add(1);
                        if !reported_missing_tool {
                            reported_missing_tool = true;
                            report.note(format!(
                                "pdftotext unavailable or failing ({error}); PDF artifacts are                                  enumerated but not read"
                            ));
                        }
                        None
                    }
                },
                ArtifactFormat::Unparsed => None,
            };
            let Some(parsed) = parsed else {
                if format != ArtifactFormat::Pdf {
                    stats.artifacts_parse_failed = stats.artifacts_parse_failed.saturating_add(1);
                }
                if format != ArtifactFormat::Pdf {
                    report.note(format!(
                        "{}: {} body did not parse as a result report",
                        artifact.url,
                        format.as_str()
                    ));
                }
                continue;
            };
            stats.artifacts_parsed = stats.artifacts_parsed.saturating_add(1);
            let parsed_formats = stats
                .formats
                .entry(format.as_str().to_string())
                .or_default();
            *parsed_formats = parsed_formats.saturating_add(1);
            let seasons = stats.seasons.entry(artifact.year).or_default();
            *seasons = seasons.saturating_add(1);
            let rows = absorb(
                &parsed,
                &artifact,
                sport,
                &options.observed_on,
                &index,
                &mut resolved,
                &mut stats,
                &mut accumulated,
            );
            ctx.store.journal_done(
                "wiaa_results",
                &artifact.url,
                &json!({
                    "url": artifact.url,
                    "parser": PARSE_VERSION,
                    "format": format.as_str(),
                    "year": artifact.year,
                    "parsed": true,
                    "meet": parsed.name,
                    "date": parsed.date,
                    "rows": rows,
                }),
            )?;
        }
    }

    // One append per table keeps the entity logs tight and the run resumable.
    let meets: Vec<CanonicalMeet> = accumulated.meets.into_values().collect();
    let teams: Vec<CanonicalTeam> = accumulated.teams.into_values().collect();
    let athletes: Vec<CanonicalAthlete> = accumulated.athletes.into_values().collect();
    let events: Vec<CanonicalEvent> = accumulated.events.into_values().collect();
    let performances: Vec<CanonicalPerformance> = accumulated.performances.into_values().collect();
    ctx.store.append_many(Table::Meets, &meets)?;
    ctx.store.append_many(Table::Teams, &teams)?;
    ctx.store.append_many(Table::Athletes, &athletes)?;
    ctx.store.append_many(Table::Events, &events)?;
    ctx.store.append_many(Table::Performances, &performances)?;

    let (requests_after, cache_after) = stats_of(ctx).await;
    report.rows = u64::try_from(stats.artifacts_parsed)
        .context("parsed artifact count does not fit in u64")?;
    report.requests = requests_after.saturating_sub(requests_before);
    report.from_cache = cache_after.saturating_sub(cache_before);
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
    report.note(format!(
        "canonical entities: meets {} events {} athletes {} teams {} performances {}",
        meets.len(),
        events.len(),
        athletes.len(),
        teams.len(),
        performances.len()
    ));
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
    Ok(report)
}
