use super::super::super::map::{absorb, AbsorbedMeet, RowWriter};
use super::super::super::{school_year_for, ArchiveArtifact, ArtifactFormat, PARSE_VERSION};
use super::super::ArtifactRun;
use crate::context::PerformanceDateAssessment;
use crate::result_file::ParsedMeet;
use crate::{AdapterReport, CrawlError, CrawlResult};
use census_domain::model::Sport;
use serde_json::json;

pub(super) struct ReadArtifact<'a> {
    pub(super) parsed: &'a ParsedMeet,
    pub(super) artifact: &'a ArchiveArtifact,
    pub(super) format: ArtifactFormat,
    pub(super) sport: Sport,
    pub(super) observed_on: &'a str,
    pub(super) date_assessment: PerformanceDateAssessment,
    pub(super) performance_as_of: chrono::NaiveDate,
    pub(super) projection_key: String,
}

pub(super) fn record_parsed_artifact(
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
    count_parsed_artifact(run, &read);
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
            note_projection_failure(report, run, &read, error);
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

fn count_parsed_artifact(run: &mut ArtifactRun, read: &ReadArtifact<'_>) {
    run.stats.artifacts_parsed = run.stats.artifacts_parsed.saturating_add(1);
    let parsed_formats = run
        .stats
        .formats
        .entry(read.format.as_str().to_string())
        .or_default();
    *parsed_formats = parsed_formats.saturating_add(1);
    let seasons = run.stats.seasons.entry(read.artifact.year).or_default();
    *seasons = seasons.saturating_add(1);
}

fn note_projection_failure(
    report: &mut AdapterReport,
    run: &mut ArtifactRun,
    read: &ReadArtifact<'_>,
    error: CrawlError,
) {
    run.stats.artifacts_parse_failed = run.stats.artifacts_parse_failed.saturating_add(1);
    report.errors = report.errors.saturating_add(1);
    report.note(format!(
        "{}: {error}; projection remains unfinished",
        read.artifact.url
    ));
    report
        .unfinished
        .push(format!("{}: {error}", read.artifact.url));
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
