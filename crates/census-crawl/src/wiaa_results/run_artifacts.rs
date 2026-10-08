use super::super::map::{absorb, AbsorbedMeet, RowWriter};
use super::super::parse::{parse_pdf, pdftotext};
use super::super::{
    artifact_format, school_year_for, ArchiveArtifact, ArtifactFormat, PARSE_VERSION,
};
use super::ArtifactRun;
use crate::context::PerformanceDateAssessment;
use crate::net::FetchOutcome;
use crate::result_file::ParsedMeet;
use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::model::{SourceRef, Sport};
use serde_json::json;

pub(super) async fn process_artifact(
    ctx: &AdapterContext<'_>,
    report: &mut AdapterReport,
    run: &mut ArtifactRun,
    artifact: &ArchiveArtifact,
    sport: Sport,
) -> CrawlResult<()> {
    let extension = artifact.extension.as_str();
    let Some(fetched) = fetch_body(ctx, report, run, artifact, extension).await else {
        return Ok(());
    };
    let projection_key = super::run_receipts::key(
        &run.projection_context,
        &artifact.url,
        &fetched.outcome.body,
    )?;
    if run.done.contains(&projection_key) {
        return Ok(());
    }
    let source = SourceRef::new("wiaa_results", Some(artifact.url.clone()));
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
            observed_on: &fetched.outcome.fetched_at,
            date_assessment: ctx.assess_performance_date(&parsed.date),
            performance_as_of: ctx.performance_as_of,
            projection_key,
        },
    )
}

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
            report.unfinished.push(format!("{}: {error}", artifact.url));
            return None;
        }
    };
    let body = fetched.text();
    let format = artifact_format(extension, Some(&body));
    if format == ArtifactFormat::Unparsed {
        run.stats.artifacts_unsupported = run.stats.artifacts_unsupported.saturating_add(1);
        report.note(format!("{}: unrecognised result format", artifact.url));
        report.unfinished.push(format!(
            "{}: unrecognized result format; raw response retained",
            artifact.url
        ));
        return None;
    }
    Some(FetchedBody {
        outcome: fetched,
        body,
        format,
    })
}

struct FetchedBody {
    outcome: FetchOutcome,
    body: String,
    format: ArtifactFormat,
}

fn note_unparsed(
    run: &mut ArtifactRun,
    report: &mut AdapterReport,
    artifact: &ArchiveArtifact,
    format: ArtifactFormat,
) {
    report.unfinished.push(format!(
        "{}: {} body has no recognized result projection",
        artifact.url,
        format.as_str()
    ));
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

struct ReadArtifact<'a> {
    parsed: &'a ParsedMeet,
    artifact: &'a ArchiveArtifact,
    format: ArtifactFormat,
    sport: Sport,
    observed_on: &'a str,
    date_assessment: PerformanceDateAssessment,
    performance_as_of: chrono::NaiveDate,
    projection_key: String,
}

fn record_parsed_artifact(
    report: &mut AdapterReport,
    run: &mut ArtifactRun,
    read: ReadArtifact<'_>,
) -> CrawlResult<()> {
    let school_year = school_year_for(&read.parsed.date, read.sport, read.artifact.year);
    if school_year.is_none() && !matches!(read.date_assessment, PerformanceDateAssessment::Future) {
        report.unfinished.push(format!(
            "{}: source date {:?} and archive season {} do not establish an academic period",
            read.artifact.url, read.parsed.date, read.artifact.year
        ));
    }
    run.stats.artifacts_parsed = run.stats.artifacts_parsed.saturating_add(1);
    let parsed_formats = run
        .stats
        .formats
        .entry(read.format.as_str().to_string())
        .or_default();
    *parsed_formats = parsed_formats.saturating_add(1);
    let seasons = run.stats.seasons.entry(read.artifact.year).or_default();
    *seasons = seasons.saturating_add(1);
    let rows = match absorb(
        AbsorbedMeet {
            parsed: read.parsed,
            artifact: read.artifact,
            sport: read.sport,
            school_year,
            observed_on: read.observed_on,
            performance_as_of: read.performance_as_of,
            date_assessment: read.date_assessment,
        },
        RowWriter {
            index: &run.index,
            resolved: &mut run.resolved,
            stats: &mut run.stats,
            accumulator: &mut run.accumulated,
        },
    ) {
        Ok(rows) => rows,
        Err(error) => {
            run.stats.artifacts_parse_failed = run.stats.artifacts_parse_failed.saturating_add(1);
            report.errors = report.errors.saturating_add(1);
            report.note(format!(
                "{}: {error}; projection remains unfinished",
                read.artifact.url
            ));
            report
                .unfinished
                .push(format!("{}: {error}", read.artifact.url));
            return Ok(());
        }
    };
    if !note_date_assessment(report, &read)
        || (school_year.is_none()
            && matches!(read.date_assessment, PerformanceDateAssessment::Admitted))
    {
        return Ok(());
    }
    run.pending.push((
        read.projection_key,
        json!({
            "url": read.artifact.url,
            "parser": PARSE_VERSION,
            "format": read.format.as_str(),
            "year": read.artifact.year,
            "parsed": true,
            "meet": read.parsed.name,
            "date": read.parsed.date,
            "rows": rows,
            "performance_as_of": read.performance_as_of,
            "date_assessment": format!("{:?}", read.date_assessment),
        }),
    ));
    Ok(())
}

fn note_date_assessment(report: &mut AdapterReport, read: &ReadArtifact<'_>) -> bool {
    match read.date_assessment {
        PerformanceDateAssessment::Admitted => true,
        PerformanceDateAssessment::Future => {
            report.note(format!("{}: published date {:?} is after performance snapshot {}; capture and source metadata retained, performances out of scope",
                read.artifact.url, read.parsed.date, read.performance_as_of));
            true
        }
        PerformanceDateAssessment::Unknown => {
            let gap = format!("{}: published date {:?} cannot be compared to performance snapshot {}; raw capture retained for temporal review",
                read.artifact.url, read.parsed.date, read.performance_as_of);
            report.note(&gap);
            report.unfinished.push(gap);
            false
        }
    }
}
