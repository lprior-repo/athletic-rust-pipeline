use super::{level_of, Accumulator, ArchiveArtifact, Stats};
use crate::context::PerformanceDateAssessment;
use crate::result_file::{ParsedEvent, ParsedMeet};
use census_domain::model::{
    CanonicalMeet, CanonicalTeam, EventId, Evidence, Gender, SchoolId, SchoolYear, SourceIdentity,
    SourceNamespace, SourceRef, Sport, TimingMethod,
};
use census_domain::school_index::SchoolIndex;
use census_domain::UsJurisdiction;
use census_store::Entity;
use std::collections::HashMap;

mod events;
#[path = "map_rows.rs"]
mod map_rows;
mod undated;

use events::record_event;
use map_rows::record_row;

pub(super) struct AbsorbedMeet<'a> {
    pub(super) parsed: &'a ParsedMeet,
    pub(super) artifact: &'a ArchiveArtifact,
    pub(super) sport: Sport,
    pub(super) school_year: Option<SchoolYear>,
    pub(super) date_assessment: PerformanceDateAssessment,
    pub(super) performance_as_of: chrono::NaiveDate,
    pub(super) observed_on: &'a str,
}

pub(super) fn absorb(
    read: AbsorbedMeet<'_>,
    mut writer: RowWriter<'_>,
) -> crate::CrawlResult<usize> {
    let (meet, evidence, timing) =
        meet_for(read.parsed, read.artifact, read.sport, read.observed_on);
    keep_meet(&mut writer, meet.clone());
    if matches!(read.date_assessment, PerformanceDateAssessment::Unknown) {
        undated::retain(&read, &mut writer)?;
        return Err(crate::CrawlError::PerformanceDateUnknown {
            published: read.parsed.date.chars().take(64).collect(),
            as_of: read.performance_as_of,
        });
    }
    if !matches!(read.date_assessment, PerformanceDateAssessment::Admitted)
        || read.school_year.is_none()
    {
        return Ok(0);
    }
    let mut outcome = Ok(0usize);
    for event in &read.parsed.events {
        let projected = project_event(&read, &mut writer, &meet, (&evidence, timing), event);
        outcome = outcome.and_then(|count| projected.map(|rows| count.saturating_add(rows)));
    }
    outcome
}

fn project_event(
    read: &AbsorbedMeet<'_>,
    writer: &mut RowWriter<'_>,
    meet: &CanonicalMeet,
    evidence: (&Evidence, TimingMethod),
    event: &ParsedEvent,
) -> crate::CrawlResult<usize> {
    let event_id = record_event(writer, meet, event, evidence.0)?;
    let context = MeetContext {
        artifact: read.artifact,
        meet,
        evidence: evidence.0,
        sport: read.sport,
        school_year: SchoolYear::from_date(&meet.date)
            .or(read.school_year)
            .ok_or_else(|| crate::CrawlError::Schema {
                url: read.artifact.url.clone(),
                detail: "published source period does not establish a school year".into(),
            })?,
        timing: evidence.1,
        event,
        observed_on: read.observed_on,
        event_id: &event_id,
    };
    Ok(event
        .rows
        .iter()
        .enumerate()
        .fold(0usize, |count, (index, row)| {
            count.saturating_add(record_row(writer, &context, row, index))
        }))
}

pub(super) struct RowWriter<'a> {
    pub(super) index: &'a SchoolIndex,
    pub(super) resolved: &'a mut HashMap<String, Option<SchoolId>>,
    pub(super) stats: &'a mut Stats,
    pub(super) accumulator: &'a mut Accumulator,
}

struct MeetContext<'a> {
    artifact: &'a ArchiveArtifact,
    meet: &'a CanonicalMeet,
    evidence: &'a Evidence,
    sport: Sport,
    school_year: SchoolYear,
    timing: TimingMethod,
    observed_on: &'a str,
    event: &'a ParsedEvent,
    event_id: &'a EventId,
}

fn meet_for(
    parsed: &ParsedMeet,
    artifact: &ArchiveArtifact,
    sport: Sport,
    observed_on: &str,
) -> (CanonicalMeet, Evidence, TimingMethod) {
    let level = level_of(&parsed.name);
    let timing = TimingMethod::Unknown;
    let mut meet = CanonicalMeet::new(
        Some(UsJurisdiction::Wisconsin),
        parsed.name.clone(),
        canonical_date(&parsed.date),
        level,
    );
    meet.end_date = parsed.end_date.clone();
    meet.sports.push(sport);
    meet.source_urls.push(artifact.url.clone());
    meet.source_identities.push(SourceIdentity::new(
        SourceNamespace::Other("wiaa_result_file".to_string()),
        artifact.url.clone(),
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
    } else if parsed.date.len() == 4 {
        meet_evidence.note = Some(format!(
            "date published only as the archive year {}; label {} ({})",
            parsed.date, artifact.label, artifact.extension
        ));
    }
    meet.evidence.push(meet_evidence.clone());
    (meet, meet_evidence, timing)
}

fn canonical_date(published: &str) -> String {
    match crate::context::published_performance_date(published) {
        Some(date) => date.format("%Y-%m-%d").to_string(),
        None => published.to_string(),
    }
}

fn keep_meet(writer: &mut RowWriter<'_>, meet: CanonicalMeet) {
    match writer.accumulator.meets.entry(meet.id.as_str().to_string()) {
        std::collections::hash_map::Entry::Vacant(entry) => {
            entry.insert(meet);
        }
        std::collections::hash_map::Entry::Occupied(mut entry) => {
            entry.get_mut().merge(meet);
        }
    }
}
fn team_for(
    teams: &mut HashMap<String, CanonicalTeam>,
    school: &SchoolId,
    sport: Sport,
    gender: Gender,
    school_year: SchoolYear,
    evidence: &Evidence,
) -> census_domain::model::TeamId {
    let key = format!(
        "{}:{sport:?}:{gender:?}:{}",
        school.as_str(),
        school_year.get()
    );
    teams
        .entry(key)
        .or_insert_with(|| {
            let id = CanonicalTeam::mint(school, sport, gender, school_year);
            CanonicalTeam {
                id,
                school: school.clone(),
                sport,
                gender,
                school_year,
                level: Some("high_school".to_string()),
                source_identities: Vec::new(),
                evidence: vec![evidence.clone()],
                retained_conflicts: Vec::new(),
            }
        })
        .id
        .clone()
}
