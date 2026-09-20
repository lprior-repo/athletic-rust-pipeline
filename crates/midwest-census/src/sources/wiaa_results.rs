//! WIAA state result archive (Tier D — official result artifacts).
//!
//! The WIAA publishes every state, sectional and regional result file it has ever released on four
//! year-partitioned archive pages (boys/girls track & field, boys/girls cross country). The files are
//! not one format:
//!
//! | format | who publishes it | parsed |
//! |---|---|---|
//! | Hy-Tek HTML (Cocoa-converted `<p>` report) | PrimeTime Timing and other Hy-Tek timers | yes |
//! | Hy-Tek plain text (`.txt`) | HS timing systems from the 2000s | yes |
//! | RaceDay Scoring HTML tables | cross-country sectionals 2010s-2020s | yes |
//! | PDF / RTF | newer state finals | indexed, not parsed |
//!
//! The archive is the cheapest place in the platform to obtain **grade-bearing official results**:
//! every Hy-Tek and RaceDay row carries the athlete's grade at the time of the meet, which is exactly
//! the class-of-2027 evidence the census needs, and none of it costs an Athletic.net request.
//!
//! Entities are minted on the same deterministic keys the roster and association adapters use —
//! athletes from `(school, name, grad year, gender)`, teams from `(school, sport, gender, school
//! year)`, meets from `(state, date, name)` — so a WIAA result file reconciles with an existing
//! canonical athlete instead of creating a parallel one.

use crate::model::{
    tag, CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalTeam,
    CompetitionLevel, Evidence, Gender, GradYear, Grade, Id, ObservedGrade, SchoolId, SchoolYear,
    SourceEventLabel, SourceIdentity, SourceNamespace, SourceRef, Sport, TimingMethod,
};
use crate::school_index::SchoolIndex;
use crate::sources::result_file::ParsedMeet;
use crate::sources::{AdapterContext, AdapterReport};
use crate::store::Table;
use anyhow::{Context, Result};

/// Bump when a parser change alters what an already-journaled artifact yields: resume entries are
/// only honoured for the current version, so a format fix re-reads the affected files.
const PARSE_VERSION: u32 = 4;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::LazyLock;

/// The four WIAA archive pages, with the sport each one publishes.
pub const ARCHIVES: [(&str, Sport); 4] = [
    (
        "https://www.wiaawi.org/sports/boys-track-field/boys-track-field-state-archive",
        Sport::OutdoorTrack,
    ),
    (
        "https://www.wiaawi.org/sports/girls-track-field/girls-track-field-state-archive",
        Sport::OutdoorTrack,
    ),
    (
        "https://www.wiaawi.org/sports/boys-cross-country/boys-cross-country-state-archive",
        Sport::CrossCountry,
    ),
    (
        "https://www.wiaawi.org/sports/girls-cross-country/girls-cross-country-state-archive",
        Sport::CrossCountry,
    ),
];

pub struct Options {
    /// Stop after this many artifacts (smoke runs).
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: String,
    /// Restrict to these archive years; empty means every year on the archive pages.
    pub seasons: Vec<i16>,
    /// Unused: the archive is a single state. Kept for the uniform provider CLI shape.
    pub states: Vec<String>,
    /// Unused: schools come from the consolidated school snapshot.
    pub school_names: Vec<String>,
}

static LINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?is)<a[^>]+href="([^"]+)"[^>]*>(.*?)</a>"#).expect("regex"));
/// Result files live under three URL shapes: `/Results/<sport>/<year>/…` (the archive's own
/// releases), `/Portals/0/PDF/Results/…` (older mirrors, matched by the same `/Results/` marker) and
/// `/sites/default/files/<year>-<month>/…` (the current-season files the state meet pages link).
static RESULT_PATH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)/(?:Results/(?:Track|Cross_Country)/(\d{4})/|sites/default/files/(\d{4})-\d{2}/)",
    )
    .expect("regex")
});
static TAGS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?is)<[^>]*>").expect("regex"));

/// One artifact link found on an archive page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArchiveArtifact {
    pub url: String,
    pub year: i16,
    /// File name without directory or extension — the artifact's local key.
    pub stem: String,
    /// Anchor text as published (`Boys`, `Team`, `Division 1`, …).
    pub label: String,
    pub extension: String,
}

/// Classify an artifact by what the platform can do with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactFormat {
    HytekHtml,
    HytekText,
    RaceDay,
    /// A PDF release: read through `pdftotext -layout`, which preserves the report's columns.
    Pdf,
    Unparsed,
}

impl ArtifactFormat {
    pub const fn as_str(self) -> &'static str {
        match self {
            ArtifactFormat::HytekHtml => "hytek_html",
            ArtifactFormat::HytekText => "hytek_text",
            ArtifactFormat::RaceDay => "raceday",
            ArtifactFormat::Pdf => "pdf",
            ArtifactFormat::Unparsed => "unparsed",
        }
    }
}

/// Decide how an artifact will be read, from its extension and (optionally) its body.
pub fn artifact_format(extension: &str, body: Option<&str>) -> ArtifactFormat {
    match extension {
        "htm" | "html" => {
            let body = body.unwrap_or_default();
            if body.contains("RaceDay Scoring") || body.contains("data-display") {
                ArtifactFormat::RaceDay
            } else {
                ArtifactFormat::HytekHtml
            }
        }
        "txt" => ArtifactFormat::HytekText,
        "pdf" => ArtifactFormat::Pdf,
        _ => ArtifactFormat::Unparsed,
    }
}

/// Extract every result-file link from an archive page, with its year from the URL path.
pub fn archive_artifacts(body: &str) -> Vec<ArchiveArtifact> {
    let mut artifacts = Vec::new();
    for captures in LINK.captures_iter(body) {
        let Some(href) = captures.get(1).map(|m| m.as_str()) else {
            continue;
        };
        let Some(year) = RESULT_PATH
            .captures(href)
            .and_then(|captures| captures.get(1).or_else(|| captures.get(2)))
            .and_then(|m| m.as_str().parse::<i16>().ok())
        else {
            continue;
        };
        let path = href.split('?').next().unwrap_or(href);
        let file = path.rsplit('/').next().unwrap_or(path);
        let (stem, extension) = match file.rsplit_once('.') {
            Some((stem, extension)) => (stem.to_string(), extension.to_ascii_lowercase()),
            None => (file.to_string(), String::new()),
        };
        let label = captures
            .get(2)
            .map(|m| {
                TAGS.replace_all(m.as_str(), "")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_default();
        let url = if href.starts_with("http") {
            href.to_string()
        } else {
            format!(
                "https://www.wiaawi.org{}",
                href.trim_start_matches("https://www.wiaawi.org")
            )
        };
        artifacts.push(ArchiveArtifact {
            url,
            year,
            stem,
            label,
            extension,
        });
    }
    artifacts.sort_by(|a, b| a.url.cmp(&b.url));
    artifacts.dedup_by(|a, b| a.url == b.url);
    artifacts
}

/// Read a PDF release through `pdftotext -layout`.
///
/// The PDF is streamed in and the text out, so no temporary file is written; the writer runs on its
/// own thread because a large PDF exceeds the pipe buffer while the parent is still reading.
fn pdftotext(body: &[u8]) -> std::io::Result<String> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new("pdftotext")
        .args(["-layout", "-", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let mut stdin = child.stdin.take().expect("stdin was piped");
    let payload = body.to_vec();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&payload);
        drop(stdin);
    });
    let output = child.wait_with_output()?;
    writer.join().ok();
    if !output.status.success() {
        return Err(std::io::Error::other(format!(
            "pdftotext exited with {}",
            output.status
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Which school year a meet date falls in.
///
/// A track season runs inside one school year (`2025-06-06` → 2024-25); a cross-country season opens
/// the next one (`2025-10-25` → 2025-26). Files that publish no date at all fall back to the archive
/// year with the sport's start month, which is why that fallback is documented rather than hidden.
pub fn school_year_for(date: &str, sport: Sport, archive_year: i16) -> SchoolYear {
    let year = date
        .get(..4)
        .and_then(|value| value.parse::<i16>().ok())
        .unwrap_or(archive_year);
    let month = date.get(5..7).and_then(|value| value.parse::<u8>().ok());
    match (month, sport) {
        (Some(month), _) => SchoolYear::containing(year, month),
        (None, Sport::CrossCountry) => SchoolYear::containing(year, 10),
        (None, _) => SchoolYear::containing(year, 6),
    }
}

/// Competition level from the meet name the result file publishes.
pub fn level_of(name: &str) -> CompetitionLevel {
    let lowered = name.to_ascii_lowercase();
    if lowered.contains("state") {
        CompetitionLevel::State
    } else if lowered.contains("sectional") {
        CompetitionLevel::Sectional
    } else if lowered.contains("regional") {
        CompetitionLevel::Regional
    } else if lowered.contains("conference") || lowered.contains("invit") {
        CompetitionLevel::Invitational
    } else {
        CompetitionLevel::Unknown
    }
}

#[derive(Debug, Default)]
struct Accumulator {
    meets: HashMap<String, CanonicalMeet>,
    events: HashMap<String, CanonicalEvent>,
    teams: HashMap<String, CanonicalTeam>,
    athletes: HashMap<String, CanonicalAthlete>,
    performances: HashMap<String, CanonicalPerformance>,
}

#[derive(Debug, Default)]
struct Stats {
    artifacts_seen: usize,
    artifacts_parsed: usize,
    artifacts_unparsed: usize,
    artifacts_unsupported: usize,
    artifacts_parse_failed: usize,
    artifacts_failed: usize,
    pdf_parsed: usize,
    pdf_unparsed: usize,
    pdf_tool_failures: usize,
    rows: usize,
    rows_with_grade: usize,
    rows_without_school: usize,
    relay_legs: usize,
    events: usize,
    school_resolved: HashMap<&'static str, usize>,
    unresolved: HashMap<String, usize>,
    formats: HashMap<String, usize>,
    seasons: HashMap<i16, usize>,
}

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
        let artifacts = archive_artifacts(&body);
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
            stats.artifacts_seen += 1;
            if done.contains(&artifact.url) {
                continue;
            }
            let extension = artifact.extension.as_str();
            if artifact_format(extension, None) == ArtifactFormat::Unparsed {
                stats.artifacts_unparsed += 1;
                *stats.formats.entry(extension.to_string()).or_default() += 1;
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
                    stats.artifacts_failed += 1;
                    report.note(format!("{}: {error}", artifact.url));
                    continue;
                }
            };
            let body = fetched.text();
            let format = artifact_format(extension, Some(&body));
            if format == ArtifactFormat::Unparsed {
                stats.artifacts_unsupported += 1;
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
                    crate::sources::raceday::parse(&body, source.clone(), artifact.year)
                }
                ArtifactFormat::Pdf => match pdftotext(&fetched.body) {
                    Ok(text) => {
                        let parsed = crate::sources::hytek::parse(
                            &crate::sources::hytek::lines_from_pdf_text(&text),
                            source.clone(),
                        );
                        if parsed.is_some() {
                            stats.pdf_parsed += 1;
                        } else {
                            stats.pdf_unparsed += 1;
                            report.note(format!(
                                "{}: pdf text is not a report this parser knows",
                                artifact.url
                            ));
                        }
                        parsed
                    }
                    Err(error) => {
                        stats.pdf_tool_failures += 1;
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
                    stats.artifacts_parse_failed += 1;
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
            stats.artifacts_parsed += 1;
            *stats
                .formats
                .entry(format.as_str().to_string())
                .or_default() += 1;
            *stats.seasons.entry(artifact.year).or_default() += 1;
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
    report.rows = stats.artifacts_parsed as u64;
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

#[allow(clippy::too_many_arguments)]
fn absorb(
    parsed: &ParsedMeet,
    artifact: &ArchiveArtifact,
    sport: Sport,
    observed_on: &str,
    index: &SchoolIndex,
    resolved: &mut HashMap<String, Option<SchoolId>>,
    stats: &mut Stats,
    accumulator: &mut Accumulator,
) -> usize {
    let level = level_of(&parsed.name);
    // The files never state a timing method. WIAA tournament rounds (regional, sectional, state) are
    // fully automatic per association policy, so the meet's level is the provenance; anything that
    // does not read as a tournament round stays `Unknown` rather than inheriting a "fast" guess.
    let timing = match level {
        CompetitionLevel::State | CompetitionLevel::Sectional | CompetitionLevel::Regional => {
            TimingMethod::Fat
        }
        _ => TimingMethod::Unknown,
    };
    let mut meet = CanonicalMeet::new("WI", parsed.name.clone(), parsed.date.clone(), level);
    meet.end_date = parsed.end_date.clone();
    meet.sports.push(sport);
    meet.source_urls.push(artifact.url.clone());
    meet.source_identities.push(SourceIdentity::new(
        SourceNamespace::Other("wiaa_result_file".to_string()),
        artifact.stem.clone(),
    ));
    let mut meet_evidence = Evidence::parsed(
        SourceRef::new("wiaa_results", Some(artifact.url.clone())),
        observed_on,
    );
    if let Some(timer) = &parsed.timer {
        meet_evidence.note = Some(format!(
            "official WIAA artifact timed by {timer}; label {} ({})",
            artifact.label, artifact.extension
        ));
    }
    meet.evidence.push(meet_evidence.clone());

    let school_year = school_year_for(&parsed.date, sport, artifact.year);
    let mut athlete_rows = 0usize;

    for parsed_event in &parsed.events {
        let mut event_entry = CanonicalEvent::new(
            &meet.id,
            parsed_event.kind.clone(),
            parsed_event.gender,
            parsed_event.division.as_deref(),
            parsed_event.round.as_deref(),
        );
        let event_id = event_entry.id.clone();
        event_entry.source_labels.push(SourceEventLabel {
            source: SourceRef::new("wiaa_results", Some(artifact.url.clone())),
            label: parsed_event.label.clone(),
        });
        event_entry.evidence.push(meet_evidence.clone());
        stats.events += 1;
        accumulator
            .events
            .entry(event_id.as_str().to_string())
            .or_insert(event_entry);

        for (row_index, row) in parsed_event.rows.iter().enumerate() {
            stats.rows += 1;
            if row.grade.is_some() {
                stats.rows_with_grade += 1;
            }
            if row.school.trim().is_empty() {
                stats.rows_without_school += 1;
                continue;
            }
            let school_id = resolved
                .entry(row.school.clone())
                .or_insert_with(|| match index.resolve("WI", &row.school) {
                    Some((id, kind)) => {
                        *stats.school_resolved.entry(kind.as_str()).or_default() += 1;
                        Some(id)
                    }
                    None => {
                        *stats.unresolved.entry(row.school.clone()).or_default() += 1;
                        None
                    }
                })
                .clone();
            let Some(school_id) = school_id else {
                continue;
            };

            // Individual rows name an athlete; relay rows name a school and list their legs.
            let members: Vec<(Option<u8>, String, Option<Grade>)> = if row.legs.is_empty() {
                vec![(None, row.name.clone(), row.grade)]
            } else {
                row.legs
                    .iter()
                    .map(|leg| (Some(leg.position), leg.name.clone(), leg.grade))
                    .collect()
            };
            if !row.legs.is_empty() {
                stats.relay_legs += row.legs.len();
            }
            let team_id = team_for(
                &mut accumulator.teams,
                &school_id,
                sport,
                parsed_event.gender,
                school_year,
                &meet_evidence,
            );

            for (leg_position, member_name, member_grade) in members {
                let Some(grade) = member_grade else {
                    continue;
                };
                if member_name.trim().is_empty() {
                    continue;
                }
                athlete_rows += 1;
                let grad_year = GradYear::of(grade, school_year);
                let athlete_id = CanonicalAthlete::mint(
                    &school_id,
                    &member_name,
                    grad_year,
                    parsed_event.gender,
                );
                let entry = accumulator
                    .athletes
                    .entry(athlete_id.as_str().to_string())
                    .or_insert_with(|| {
                        let mut athlete = CanonicalAthlete::new(
                            &school_id,
                            &member_name,
                            grad_year,
                            parsed_event.gender,
                        );
                        athlete.sports.push(sport);
                        athlete
                    });
                if !entry.sports.contains(&sport) {
                    entry.sports.push(sport);
                }
                let observation = ObservedGrade {
                    grade,
                    school_year,
                    source: SourceRef::new("wiaa_results", Some(artifact.url.clone())),
                };
                if !entry.observed_grades.contains(&observation) {
                    entry.observed_grades.push(observation);
                }
                if !entry
                    .evidence
                    .iter()
                    .any(|existing| existing == &meet_evidence)
                {
                    entry.evidence.push(meet_evidence.clone());
                }

                let source_key = match leg_position {
                    Some(position) => format!(
                        "{}:{}:{}:{}:leg{}",
                        artifact.stem,
                        parsed_event.label,
                        parsed_event.round.as_deref().unwrap_or("final"),
                        row_index,
                        position
                    ),
                    None => format!(
                        "{}:{}:{}:{}",
                        artifact.stem,
                        parsed_event.label,
                        parsed_event.round.as_deref().unwrap_or("final"),
                        row_index
                    ),
                };
                let performance_id = CanonicalPerformance::mint(
                    &athlete_id,
                    &meet.id,
                    &parsed_event.kind,
                    &meet.date,
                    &source_key,
                );
                let mut evidence = meet_evidence.clone();
                if let Some(position) = leg_position {
                    evidence.note = Some(format!(
                        "relay leg {position} for {}{}; the published mark is the team's",
                        row.school,
                        row.heat
                            .as_deref()
                            .map(|heat| format!(" squad {heat}"))
                            .unwrap_or_default()
                    ));
                }
                accumulator
                    .performances
                    .entry(performance_id.as_str().to_string())
                    .or_insert_with(|| CanonicalPerformance {
                        id: performance_id,
                        athlete: athlete_id,
                        team: team_id.clone(),
                        event: event_id.clone(),
                        meet: meet.id.clone(),
                        date: meet.date.clone(),
                        mark: row.mark.clone(),
                        wind_mps: row.wind_mps,
                        place: row.place,
                        heat: row.heat.clone(),
                        round: parsed_event.round.clone(),
                        timing: Some(timing),
                        observed_grade: Some(grade),
                        evidence: vec![evidence],
                        source_key,
                    });
            }
        }
    }
    accumulator
        .meets
        .entry(meet.id.as_str().to_string())
        .or_insert(meet);
    athlete_rows
}

fn team_for(
    teams: &mut HashMap<String, CanonicalTeam>,
    school: &SchoolId,
    sport: Sport,
    gender: Gender,
    school_year: SchoolYear,
    evidence: &Evidence,
) -> crate::model::TeamId {
    let key = format!(
        "{}:{sport:?}:{gender:?}:{}",
        school.as_str(),
        school_year.start_year()
    );
    teams
        .entry(key)
        .or_insert_with(|| {
            let id = Id::<tag::Team>::mint(
                "team",
                &[
                    school.as_str(),
                    &format!("{sport:?}"),
                    &format!("{gender:?}"),
                    &school_year.start_year().to_string(),
                ],
            );
            CanonicalTeam {
                id,
                school: school.clone(),
                sport,
                gender,
                school_year,
                level: Some("high_school".to_string()),
                source_identities: Vec::new(),
                evidence: vec![evidence.clone()],
            }
        })
        .id
        .clone()
}

async fn stats_of(ctx: &AdapterContext<'_>) -> (u64, u64) {
    let stats = ctx.fetcher.stats().await;
    (stats.requests, stats.cache_hits)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ARCHIVE_HTML: &str = r#"
        <h3><strong>2025 Track &amp; Field State Results</strong></h3>
        <ul><li>Division 1 - <a href="/Portals/0/PDF/Results/Track/2025/d1boysstateresults.htm">Boys</a>
        | <a href="https://www.wiaawi.org/Portals/0/PDF/Results/Track/2025/d1girlsstateresults.htm">Girls</a></li></ul>
        <h3><strong>2025 Track &amp; Field Sectional Results</strong></h3>
        <ul><li>Neenah - <a href="/Portals/0/PDF/Results/Track/2025/neenahsectionalb.pdf">Boys</a></li></ul>
        <a href="/sports/boys-track-field/boys-track-field">Sport home</a>
    "#;

    #[test]
    fn archive_links_carry_year_stem_label_and_extension() {
        let artifacts = archive_artifacts(ARCHIVE_HTML);
        assert_eq!(
            artifacts.len(),
            3,
            "only result-file links are artifacts: {artifacts:?}"
        );
        let first = &artifacts[0];
        assert_eq!(first.year, 2025);
        assert_eq!(first.stem, "d1boysstateresults");
        assert_eq!(first.extension, "htm");
        assert_eq!(first.label, "Boys");
        assert!(
            artifacts
                .iter()
                .all(|artifact| artifact.url.starts_with("https://www.wiaawi.org/")),
            "relative hrefs are resolved against the archive host"
        );
        assert!(artifacts.iter().any(|artifact| artifact.extension == "pdf"));
    }

    #[test]
    fn formats_are_decided_by_extension_and_sniffed_body() {
        assert_eq!(artifact_format("pdf", None), ArtifactFormat::Pdf);
        assert_eq!(artifact_format("rtf", None), ArtifactFormat::Unparsed);
        assert_eq!(artifact_format("txt", None), ArtifactFormat::HytekText);
        assert_eq!(
            artifact_format("htm", Some("<p>HY-TEK's Meet Manager</p>")),
            ArtifactFormat::HytekHtml
        );
        assert_eq!(
            artifact_format("htm", Some("<title>RaceDay Scoring</title>")),
            ArtifactFormat::RaceDay
        );
    }

    #[test]
    fn current_season_files_are_found_under_the_dated_upload_path() {
        // The state meet pages link the newest releases as `/sites/default/files/<year>-<month>/…`,
        // which carries no `/Results/` segment; missing them costs the whole current season.
        let body = r#"
            <a href="/sites/default/files/2026-08/trb2026d1stateresults.pdf">Boys</a>
            <a href="/sites/default/files/2026-08/tr2026arrowheadregionalindiv.pdf">Arrowhead</a>
            <a href="/sites/default/files/2026-08/tr2026beaverdamsectionalindiv.pdf">Beaver Dam</a>
            <a href="/sites/default/files/2026-08/somethingelse.pdf">Unrelated upload</a>
            <a href="/sites/default/files/2025-11/xcstate.htm">XC state</a>
        "#;
        let artifacts = archive_artifacts(body);
        assert_eq!(
            artifacts.len(),
            5,
            "every dated upload is treated as a candidate result file: {artifacts:?}"
        );
        assert!(artifacts
            .iter()
            .all(|artifact| artifact.year == 2026 || artifact.year == 2025));
        let state = artifacts
            .iter()
            .find(|artifact| artifact.stem == "trb2026d1stateresults")
            .expect("the 2026 state file is discovered");
        assert_eq!(state.year, 2026);
        assert_eq!(state.extension, "pdf");
        assert_eq!(state.label, "Boys");
    }

    #[test]
    fn school_year_follows_the_sport_boundary() {
        // Spring 2025 track is inside school year 2024-25: grade 11 there is class of 2026.
        let spring = school_year_for("2025-06-06", Sport::OutdoorTrack, 2025);
        assert_eq!(spring.start_year(), 2024);
        assert_eq!(GradYear::of(Grade::new(11).unwrap(), spring).0, 2026);
        // Fall 2025 cross country opens school year 2025-26: grade 11 there is class of 2027.
        let fall = school_year_for("2025-10-25", Sport::CrossCountry, 2025);
        assert_eq!(fall.start_year(), 2025);
        assert_eq!(GradYear::of(Grade::new(11).unwrap(), fall).0, 2027);
        // A year-only date (RaceDay) still lands in the right school year per sport.
        assert_eq!(
            school_year_for("2023", Sport::CrossCountry, 2023).start_year(),
            2023
        );
        assert_eq!(
            school_year_for("2023", Sport::OutdoorTrack, 2023).start_year(),
            2022
        );
    }

    #[test]
    fn meet_levels_come_from_the_published_name() {
        assert_eq!(
            level_of("WIAA Track & Field State Championships"),
            CompetitionLevel::State
        );
        assert_eq!(
            level_of("WIAA D2 XC Sectionals - Boys Race"),
            CompetitionLevel::Sectional
        );
        assert_eq!(level_of("Arrowhead Regional"), CompetitionLevel::Regional);
        assert_eq!(
            level_of("Some Invitational"),
            CompetitionLevel::Invitational
        );
        assert_eq!(level_of("Dual Meet"), CompetitionLevel::Unknown);
    }
}
