//! The per-artifact stage of the WIAA results walk: fetch one artifact, read the format it
//! declares, and absorb what it yields.

use super::super::map::absorb;
use super::super::parse::{parse_pdf, pdftotext};
use super::super::{artifact_format, ArchiveArtifact, ArtifactFormat, Options, PARSE_VERSION};
use super::ArtifactRun;
use census_domain::model::{SourceRef, Sport};
use crate::net::FetchOutcome;
use crate::sources::result_file::ParsedMeet;
use crate::sources::{AdapterContext, AdapterReport};
use anyhow::Result;
use serde_json::json;

/// Read one artifact: index it when its extension is unparsable, otherwise parse and absorb it.
pub(super) async fn process_artifact(
    ctx: &AdapterContext<'_>,
    options: &Options,
    report: &mut AdapterReport,
    run: &mut ArtifactRun,
    artifact: &ArchiveArtifact,
    sport: Sport,
) -> Result<()> {
    let extension = artifact.extension.as_str();
    if artifact_format(extension, None) == ArtifactFormat::Unparsed {
        run.stats.artifacts_unparsed = run.stats.artifacts_unparsed.saturating_add(1);
        let formats = run.stats.formats.entry(extension.to_string()).or_default();
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
        return Ok(());
    }

    let fetched = match ctx.fetcher.get(&artifact.url, &ctx.fetch_options()).await {
        Ok(meta) => meta,
        Err(error) => {
            run.stats.artifacts_failed = run.stats.artifacts_failed.saturating_add(1);
            report.note(format!("{}: {error}", artifact.url));
            return Ok(());
        }
    };
    let body = fetched.text();
    let format = artifact_format(extension, Some(&body));
    if format == ArtifactFormat::Unparsed {
        run.stats.artifacts_unsupported = run.stats.artifacts_unsupported.saturating_add(1);
        report.note(format!("{}: unrecognised result format", artifact.url));
        return Ok(());
    }
    let source = run.source.clone();
    let Some(parsed) = parse_artifact(&fetched, &body, format, artifact, &source, run, report)
    else {
        note_unparsed(run, report, artifact, format);
        return Ok(());
    };
    record_parsed_artifact(
        ctx,
        run,
        &parsed,
        artifact,
        format,
        sport,
        &options.observed_on,
    )
}

/// Note a body that yielded no meet: counted unless PDFs are already reported by their tool error.
fn note_unparsed(
    run: &mut ArtifactRun,
    report: &mut AdapterReport,
    artifact: &ArchiveArtifact,
    format: ArtifactFormat,
) {
    if format != ArtifactFormat::Pdf {
        run.stats.artifacts_parse_failed = run.stats.artifacts_parse_failed.saturating_add(1);
    }
    if format != ArtifactFormat::Pdf {
        report.note(format!(
            "{}: {} body did not parse as a result report",
            artifact.url,
            format.as_str()
        ));
    }
}

/// Dispatch one artifact body to the reader its format selects.
fn parse_artifact(
    fetched: &FetchOutcome,
    body: &str,
    format: ArtifactFormat,
    artifact: &ArchiveArtifact,
    source: &SourceRef,
    run: &mut ArtifactRun,
    report: &mut AdapterReport,
) -> Option<ParsedMeet> {
    match format {
        ArtifactFormat::HytekHtml => crate::sources::hytek::parse(
            &crate::sources::hytek::lines_from_html(body),
            source.clone(),
        ),
        ArtifactFormat::HytekText => crate::sources::hytek::parse(
            &crate::sources::hytek::lines_from_text(body),
            source.clone(),
        ),
        ArtifactFormat::RaceDay => {
            // RaceDay reports its own typed parse error; for this runner a body that is not a
            // RaceDay report is the same thing the other arms return as `None` — an artifact
            // that yielded no meet, reported below as an unparsed body. The cause goes to the
            // run notes so the failure is diagnosable rather than only counted.
            match crate::sources::raceday::parse(body, source.clone(), artifact.year) {
                Ok(parsed) => Some(parsed),
                Err(error) => {
                    report.note(format!("{}: {error}", artifact.url));
                    None
                }
            }
        }
        ArtifactFormat::Pdf => parse_pdf_artifact(fetched, artifact, source, run, report),
        ArtifactFormat::Unparsed => None,
    }
}

/// Read a PDF artifact: extract its text, hand it to the PDF parser, and report the outcome.
fn parse_pdf_artifact(
    fetched: &FetchOutcome,
    artifact: &ArchiveArtifact,
    source: &SourceRef,
    run: &mut ArtifactRun,
    report: &mut AdapterReport,
) -> Option<ParsedMeet> {
    match pdftotext(&fetched.body) {
        Ok(text) => {
            let (parsed, layout) = parse_pdf(&text, source.clone(), artifact.year);
            if let Some(layout) = layout {
                run.stats.pdf_parsed = run.stats.pdf_parsed.saturating_add(1);
                let layouts = run.stats.pdf_layouts.entry(layout.to_string()).or_default();
                *layouts = layouts.saturating_add(1);
            } else {
                run.stats.pdf_unparsed = run.stats.pdf_unparsed.saturating_add(1);
                report.note(format!(
                    "{}: pdf text is not a report this parser knows",
                    artifact.url
                ));
            }
            parsed
        }
        Err(error) => {
            run.stats.pdf_tool_failures = run.stats.pdf_tool_failures.saturating_add(1);
            if !run.reported_missing_tool {
                run.reported_missing_tool = true;
                report.note(format!(
                    "pdftotext unavailable or failing ({error}); PDF artifacts are                                  enumerated but not read"
                ));
            }
            None
        }
    }
}

/// Count a parsed artifact, absorb its meet, and journal the body as read.
fn record_parsed_artifact(
    ctx: &AdapterContext<'_>,
    run: &mut ArtifactRun,
    parsed: &ParsedMeet,
    artifact: &ArchiveArtifact,
    format: ArtifactFormat,
    sport: Sport,
    observed_on: &str,
) -> Result<()> {
    run.stats.artifacts_parsed = run.stats.artifacts_parsed.saturating_add(1);
    let parsed_formats = run
        .stats
        .formats
        .entry(format.as_str().to_string())
        .or_default();
    *parsed_formats = parsed_formats.saturating_add(1);
    let seasons = run.stats.seasons.entry(artifact.year).or_default();
    *seasons = seasons.saturating_add(1);
    let rows = absorb(
        parsed,
        artifact,
        sport,
        observed_on,
        &run.index,
        &mut run.resolved,
        &mut run.stats,
        &mut run.accumulated,
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
    Ok(())
}
