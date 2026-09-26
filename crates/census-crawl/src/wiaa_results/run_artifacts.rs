//! The per-artifact stage of the WIAA results walk: fetch one artifact, read the format it
//! declares, and absorb what it yields.

use super::super::map::{absorb, AbsorbedMeet, RowWriter};
use super::super::parse::{parse_pdf, pdftotext};
use super::super::{
    artifact_format, school_year_for, ArchiveArtifact, ArtifactFormat, Options, PARSE_VERSION,
};
use super::ArtifactRun;
use crate::net::FetchOutcome;
use crate::result_file::ParsedMeet;
use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::model::{SourceRef, Sport};
use serde_json::json;

/// Read one artifact: index it when its extension is unparsable, otherwise parse and absorb it.
pub(super) async fn process_artifact(
    ctx: &AdapterContext<'_>,
    options: &Options,
    report: &mut AdapterReport,
    run: &mut ArtifactRun,
    artifact: &ArchiveArtifact,
    sport: Sport,
) -> CrawlResult<()> {
    let extension = artifact.extension.as_str();
    if artifact_format(extension, None) == ArtifactFormat::Unparsed {
        return index_unparsed(run, artifact, extension);
    }
    let Some(fetched) = fetch_body(ctx, report, run, artifact, extension).await else {
        return Ok(());
    };
    let source = run.source.clone();
    let Some(parsed) = parse_artifact(
        &fetched.outcome,
        &fetched.body,
        fetched.format,
        artifact,
        &source,
        run,
        report,
    ) else {
        note_unparsed(run, report, artifact, fetched.format);
        return Ok(());
    };
    record_parsed_artifact(
        report,
        run,
        ReadArtifact {
            parsed: &parsed,
            artifact,
            format: fetched.format,
            sport,
            observed_on: &options.observed_on,
        },
    )
}

/// Index an artifact whose extension names no format this walk reads: counted, journalled, never
/// fetched.
///
/// The journal entry is what keeps a later run from looking at the same bytes twice: the artifact is
/// recorded as read at the version of the index that looked at it.
fn index_unparsed(
    run: &mut ArtifactRun,
    artifact: &ArchiveArtifact,
    extension: &str,
) -> CrawlResult<()> {
    run.stats.artifacts_unparsed = run.stats.artifacts_unparsed.saturating_add(1);
    let formats = run.stats.formats.entry(extension.to_string()).or_default();
    *formats = formats.saturating_add(1);
    run.pending.push((
        artifact.url.clone(),
        json!({
            "url": artifact.url,
            "parser": PARSE_VERSION,
            "format": "indexed_only",
            "year": artifact.year,
            "stem": artifact.stem
        }),
    ));
    Ok(())
}

/// Fetch an artifact's body and the format its bytes declare, or `None` when the walk skips it.
///
/// Both skips are counted and reported here rather than by the caller: a fetch that failed, and a
/// body that neither its extension nor its own bytes place in a format this walk reads.
async fn fetch_body(
    ctx: &AdapterContext<'_>,
    report: &mut AdapterReport,
    run: &mut ArtifactRun,
    artifact: &ArchiveArtifact,
    extension: &str,
) -> Option<FetchedBody> {
    let fetched = match ctx.fetcher.get(&artifact.url, &ctx.fetch_options()).await {
        Ok(meta) => meta,
        Err(error) => {
            run.stats.artifacts_failed = run.stats.artifacts_failed.saturating_add(1);
            report.note(format!("{}: {error}", artifact.url));
            return None;
        }
    };
    let body = fetched.text();
    let format = artifact_format(extension, Some(&body));
    if format == ArtifactFormat::Unparsed {
        run.stats.artifacts_unsupported = run.stats.artifacts_unsupported.saturating_add(1);
        report.note(format!("{}: unrecognised result format", artifact.url));
        return None;
    }
    Some(FetchedBody {
        outcome: fetched,
        body,
        format,
    })
}

/// A fetched body with the format it declares: the bytes the parse reads, and the format the
/// absorption journals.
struct FetchedBody {
    outcome: FetchOutcome,
    body: String,
    format: ArtifactFormat,
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
        ArtifactFormat::HytekHtml => {
            crate::hytek::parse(&crate::hytek::lines_from_html(body), source.clone())
        }
        ArtifactFormat::HytekText => {
            crate::hytek::parse(&crate::hytek::lines_from_text(body), source.clone())
        }
        ArtifactFormat::RaceDay => {
            match crate::raceday::parse(body, source.clone(), artifact.year) {
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

/// One artifact body that parsed, with the run facts its absorption needs: the sport the walk is
/// reading, the day it observed the body, and the format that produced the meet.
struct ReadArtifact<'a> {
    parsed: &'a ParsedMeet,
    artifact: &'a ArchiveArtifact,
    format: ArtifactFormat,
    sport: Sport,
    observed_on: &'a str,
}

/// Count a parsed artifact, absorb its meet, and journal the body as read.
fn record_parsed_artifact(
    report: &mut AdapterReport,
    run: &mut ArtifactRun,
    read: ReadArtifact<'_>,
) -> CrawlResult<()> {
    let Some(school_year) = school_year_for(&read.parsed.date, read.sport, read.artifact.year)
    else {
        run.stats.artifacts_parse_failed = run.stats.artifacts_parse_failed.saturating_add(1);
        report.note(format!(
            "{}: date {:?} (archive year {}) is not a school year",
            read.artifact.url, read.parsed.date, read.artifact.year
        ));
        return Ok(());
    };
    run.stats.artifacts_parsed = run.stats.artifacts_parsed.saturating_add(1);
    let parsed_formats = run
        .stats
        .formats
        .entry(read.format.as_str().to_string())
        .or_default();
    *parsed_formats = parsed_formats.saturating_add(1);
    let seasons = run.stats.seasons.entry(read.artifact.year).or_default();
    *seasons = seasons.saturating_add(1);
    let rows = absorb(
        AbsorbedMeet {
            parsed: read.parsed,
            artifact: read.artifact,
            sport: read.sport,
            school_year,
            observed_on: read.observed_on,
        },
        RowWriter {
            index: &run.index,
            resolved: &mut run.resolved,
            stats: &mut run.stats,
            accumulator: &mut run.accumulated,
        },
    );
    run.pending.push((
        read.artifact.url.clone(),
        json!({
            "url": read.artifact.url,
            "parser": PARSE_VERSION,
            "format": read.format.as_str(),
            "year": read.artifact.year,
            "parsed": true,
            "meet": read.parsed.name,
            "date": read.parsed.date,
            "rows": rows,
        }),
    ));
    Ok(())
}
