//! The document-level folds: one capture becomes the events, teams, athletes and performances it
//! publishes.
//!
//! Three folds, one per route in [`super::super::wire`]. Only an event document mints an event: the
//! summary exists to say which event ids a meet lists, and a live-standings payload names its race
//! through the run id an event document published, so it folds only into an event this run has read.

use super::super::docs::{parse_event_document, parse_event_summary, EventDoc};
use super::super::map::{Accumulator, ResultStats, RowContext, Writer, SOURCE_ID};
use super::super::map_rows::{record_row, record_standing};
use super::super::standings::parse_standings;
use super::super::wire::{event_doc_url, event_summary_url, standings_url};
use crate::school_index::SchoolIndex;
use crate::sources::athleticlive_athletes::{gender_from_token, sport_for, MeetTarget};
use crate::sources::hytek::round_marker;
use census_domain::model::{
    CanonicalEvent, CanonicalMeet, EventId, EventKind, Evidence, Gender, SchoolId, SchoolYear,
    SourceEventLabel, SourceRef,
};
use std::collections::HashMap;

/// One event a document minted in this run, keyed by the run key its standings are published under.
#[derive(Debug, Clone)]
pub(super) struct PublishedEvent {
    /// The published event id, which is the `ind_res_list/_doc/<id>` path segment.
    pub capture_id: u64,
    pub id: EventId,
    pub kind: EventKind,
    pub gender: Gender,
    pub round: Option<String>,
    /// The run key the event published (`rui`), when it published one.
    pub run_id: Option<String>,
    /// `athleticlive:<event id>`: the prefix of every row key this event mints.
    pub event_key: String,
}

/// The key one event's rows are minted under, shared by both routes that can publish its race.
fn event_key(capture_id: u64) -> String {
    format!("athleticlive:{capture_id}")
}

/// What one fold needs beyond its own payload: the meet it belongs to and the walk's own state.
pub(super) struct Fold<'a> {
    pub meet: &'a CanonicalMeet,
    pub target: &'a MeetTarget,
    pub observed_on: &'a str,
    /// The school year the meet's date sits in, which every published grade is read against.
    pub school_year: SchoolYear,
    pub index: &'a SchoolIndex,
    pub resolved: &'a mut HashMap<String, Option<SchoolId>>,
    pub stats: &'a mut ResultStats,
    pub accumulator: &'a mut Accumulator,
    /// Captures that could not be folded, in the order they were read.
    pub failures: &'a mut Vec<String>,
}

impl Fold<'_> {
    /// The row context and the row writer for one document's rows, split in one step.
    ///
    /// Both borrow this fold, so they are handed out together: the context reads the meet, the
    /// tenant and the jurisdiction, the writer owns the memo, the counters and the entities.
    fn split_for<'b>(
        &'b mut self,
        source: &'b SourceRef,
        evidence: &'b Evidence,
        event: &'b PublishedEvent,
        kind: &'b EventKind,
        sport: census_domain::model::Sport,
    ) -> (RowContext<'b>, Writer<'b>) {
        let context = RowContext {
            meet: self.meet,
            source,
            evidence,
            event_id: &event.id,
            kind,
            gender: event.gender,
            round: event.round.clone(),
            sport,
            school_year: self.school_year,
            event_key: event.event_key.clone(),
            provider: &self.target.tenant,
            jurisdiction: self.target.state,
        };
        let writer = Writer {
            index: self.index,
            resolved: &mut *self.resolved,
            stats: &mut *self.stats,
            accumulator: &mut *self.accumulator,
        };
        (context, writer)
    }
}

/// Fold one event document: mint its event, then map every row it publishes.
///
/// Returns the event it minted, or `None` when the capture names another meet, carries no event id,
/// or does not decode; the refusal is recorded on `failures`.
pub(super) fn absorb_document(
    fold: &mut Fold<'_>,
    path: &str,
    body: &str,
) -> Option<PublishedEvent> {
    let doc = match parse_event_document(path, body) {
        Ok(doc) => doc,
        Err(error) => {
            fold.failures.push(format!("{path}: {error}"));
            return None;
        }
    };
    let Some(capture_id) = doc.event_id() else {
        fold.failures
            .push(format!("{path}: the document carries no event id"));
        return None;
    };
    // A capture of another meet would file its rows under this meet's schools and date.
    if let Some(meet_id) = doc.meet_id() {
        if meet_id != fold.target.athleticlive_meet_id {
            fold.failures.push(format!(
                "{path}: event {capture_id} belongs to meet {meet_id}, not to meet {}",
                fold.target.athleticlive_meet_id
            ));
            return None;
        }
    }
    let url = event_doc_url(capture_id);
    let kind = doc.kind();
    let gender = doc
        .gender_group
        .as_deref()
        .map(gender_from_token)
        .unwrap_or(Gender::Unknown);
    let event = mint_event(fold, &doc, capture_id, &url, &kind, gender);
    fold_rows(fold, &doc, &url, &event, &kind);
    fold.stats.documents_read = fold.stats.documents_read.saturating_add(1);
    Some(event)
}

/// Mint one document's event and record the published label it was mapped from.
fn mint_event(
    fold: &mut Fold<'_>,
    doc: &EventDoc,
    capture_id: u64,
    url: &str,
    kind: &EventKind,
    gender: Gender,
) -> PublishedEvent {
    let round = doc
        .round_name
        .as_deref()
        .map(str::trim)
        .and_then(round_marker)
        .map(str::to_string);
    let source = SourceRef::new(SOURCE_ID, Some(url.to_string()));
    let mut evidence = Evidence::parsed(source.clone(), fold.observed_on);
    evidence.note = Some(format!(
        "event {capture_id} `{}`: published label `{}` mapped to {kind:?}",
        doc.name.as_deref().unwrap_or_default(),
        doc.label().unwrap_or_default()
    ));
    let event = CanonicalEvent::new(
        &fold.meet.id,
        kind.clone(),
        gender,
        doc.division_name(),
        round.as_deref(),
    );
    let entry = fold
        .accumulator
        .events
        .entry(event.id.as_str().to_string())
        .or_insert_with(|| event.clone());
    if let Some(label) = doc.label() {
        let source_label = SourceEventLabel {
            source: source.clone(),
            label: label.to_string(),
        };
        if !entry.source_labels.contains(&source_label) {
            entry.source_labels.push(source_label);
        }
    }
    if !entry.evidence.contains(&evidence) {
        entry.evidence.push(evidence);
    }
    PublishedEvent {
        capture_id,
        id: event.id,
        kind: kind.clone(),
        gender,
        round,
        run_id: doc.run_id().map(str::to_string),
        event_key: event_key(capture_id),
    }
}

/// Map every row one document publishes through the row mapper.
fn fold_rows(
    fold: &mut Fold<'_>,
    doc: &EventDoc,
    url: &str,
    event: &PublishedEvent,
    kind: &EventKind,
) {
    let source = SourceRef::new(SOURCE_ID, Some(url.to_string()));
    let mut evidence = Evidence::parsed(source.clone(), fold.observed_on);
    evidence.note = Some(format!(
        "{}: {} rows published for this event",
        event.event_key,
        doc.rows.len()
    ));
    let sport = sport_for(doc.is_xc(), &fold.target.name, &fold.target.date);
    let (context, mut writer) = fold.split_for(&source, &evidence, event, kind, sport);
    for (row_index, row) in doc.rows.iter().enumerate() {
        record_row(&mut writer, &context, row, row_index);
    }
}

/// Fold one meet's event summary: count the events it lists and return the individual ones a
/// document is expected for.
///
/// Nothing is minted from the listing: the summary states an event's name and run key, not its
/// result rows, so the document that carries them is what mints the event. `None` means the payload
/// did not decode, so the capture is not journaled and the next run reads it again.
pub(super) fn absorb_summary(fold: &mut Fold<'_>, path: &str, body: &str) -> Option<Vec<u64>> {
    let url = event_summary_url(fold.target.athleticlive_meet_id);
    let events = match parse_event_summary(&url, body) {
        Ok(events) => events,
        Err(error) => {
            fold.failures.push(format!("{path}: {error}"));
            return None;
        }
    };
    let mut fetchable: Vec<u64> = Vec::new();
    for event in events {
        fold.stats.events_listed = fold.stats.events_listed.saturating_add(1);
        if event.is_relay() {
            fold.stats.events_relay = fold.stats.events_relay.saturating_add(1);
            continue;
        }
        if matches!(event.kind(), EventKind::Unmapped { .. }) {
            fold.stats.events_unmapped = fold.stats.events_unmapped.saturating_add(1);
        }
        if let Some(event_id) = event.event_id() {
            fetchable.push(event_id);
        }
    }
    Some(fetchable)
}

/// Fold one live-standings capture into the event its run key names.
///
/// Returns the rows read, or `None` when the run key is not a usable path segment or the payload
/// does not decode; either refusal is recorded on `failures`.
pub(super) fn absorb_standings(
    fold: &mut Fold<'_>,
    run_id: &str,
    path: &str,
    body: &str,
    event: &PublishedEvent,
) -> Option<usize> {
    let Some(url) = standings_url(fold.target.athleticlive_meet_id, run_id) else {
        fold.failures
            .push(format!("{path}: run key `{run_id}` is not a path segment"));
        return None;
    };
    let rows = match parse_standings(&url, body) {
        Ok(rows) => rows,
        Err(error) => {
            fold.failures.push(format!("{path}: {error}"));
            return None;
        }
    };
    let source = SourceRef::new(SOURCE_ID, Some(url));
    let mut evidence = Evidence::parsed(source.clone(), fold.observed_on);
    evidence.note = Some(format!(
        "{}: {} standings rows published for run {run_id}",
        event.event_key,
        rows.len()
    ));
    // A standings payload publishes no event of its own: the event document that named this run key
    // is what places the race, and its `xc` marker is what makes the sport cross-country.
    let sport = sport_for(false, &fold.target.name, &fold.target.date);
    let (context, mut writer) = fold.split_for(&source, &evidence, event, &event.kind, sport);
    for (row_index, (_run_row, row)) in rows.iter().enumerate() {
        record_standing(&mut writer, &context, row, row_index);
    }
    fold.stats.standings_read = fold.stats.standings_read.saturating_add(1);
    Some(rows.len())
}
