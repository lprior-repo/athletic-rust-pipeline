use super::{level_of, Accumulator, ArchiveArtifact, Stats};
use crate::result_file::{ParsedEvent, ParsedMeet};
use census_domain::model::{
    CanonicalEvent, CanonicalMeet, CompetitionLevel, EventId, Evidence, SchoolId, SchoolYear,
    SourceEventLabel, SourceIdentity, SourceNamespace, SourceRef, Sport, TimingMethod,
};
use census_domain::school_index::SchoolIndex;
use census_domain::UsJurisdiction;
use std::collections::HashMap;

#[path = "map_rows.rs"]
mod map_rows;

use map_rows::record_row;

pub(super) struct AbsorbedMeet<'a> {
    pub(super) parsed: &'a ParsedMeet,
    pub(super) artifact: &'a ArchiveArtifact,
    pub(super) sport: Sport,
    pub(super) school_year: SchoolYear,
    pub(super) observed_on: &'a str,
}

pub(super) fn absorb(read: AbsorbedMeet<'_>, mut writer: RowWriter<'_>) -> usize {
    let AbsorbedMeet {
        parsed,
        artifact,
        sport,
        school_year,
        observed_on,
    } = read;
    let (meet, meet_evidence, timing) = meet_for(parsed, artifact, sport, observed_on);

    let mut athlete_rows = 0usize;
    for parsed_event in &parsed.events {
        let event_id = record_event(&mut writer, &meet, parsed_event, artifact, &meet_evidence);
        let context = MeetContext {
            artifact,
            meet: &meet,
            evidence: &meet_evidence,
            sport,
            school_year,
            timing,
            event: parsed_event,
            event_id: &event_id,
        };
        for (row_index, row) in parsed_event.rows.iter().enumerate() {
            athlete_rows =
                athlete_rows.saturating_add(record_row(&mut writer, &context, row, row_index));
        }
    }

    keep_meet(&mut writer, meet);
    athlete_rows
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
    let timing = match level {
        CompetitionLevel::State | CompetitionLevel::Sectional | CompetitionLevel::Regional => {
            TimingMethod::Fat
        }
        _ => TimingMethod::Unknown,
    };
    let mut meet = CanonicalMeet::new(
        Some(UsJurisdiction::Wisconsin),
        parsed.name.clone(),
        parsed.date.clone(),
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

fn record_event(
    writer: &mut RowWriter<'_>,
    meet: &CanonicalMeet,
    parsed_event: &ParsedEvent,
    artifact: &ArchiveArtifact,
    evidence: &Evidence,
) -> EventId {
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
    event_entry.evidence.push(evidence.clone());
    writer.stats.events = writer.stats.events.saturating_add(1);
    writer
        .accumulator
        .events
        .entry(event_id.as_str().to_string())
        .or_insert(event_entry);
    event_id
}

fn keep_meet(writer: &mut RowWriter<'_>, meet: CanonicalMeet) {
    writer
        .accumulator
        .meets
        .entry(meet.id.as_str().to_string())
        .or_insert(meet);
}
