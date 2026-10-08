use super::super::docs::{parse_event_summary, EventDoc, SummaryEvent};
use super::super::map::{Accumulator, ResultStats, RowContext, Writer, SOURCE_ID};
use super::super::map_rows::{record_row, record_standing};
use super::super::standings::parse_standings;
use super::super::wire::{event_doc_url, event_summary_url, standings_url};
use crate::athleticlive_athletes::{sport_for, MeetTarget};
use census_domain::model::{
    CanonicalMeet, EventId, EventKind, Evidence, Gender, SchoolId, SchoolYear, SourceRef,
};
use census_domain::school_index::SchoolIndex;
use std::collections::HashMap;
mod events;
use events::mint_event;

#[derive(Debug, Clone)]
pub(super) struct PublishedEvent {
    pub capture_id: u64,
    pub id: EventId,
    pub kind: EventKind,
    pub gender: Gender,
    pub round: Option<String>,
    pub run_id: Option<String>,
    pub event_key: String,
}

fn event_key(capture_id: u64) -> String {
    format!("athleticlive:{capture_id}")
}

pub(super) struct Fold<'a> {
    pub meet: &'a CanonicalMeet,
    pub target: &'a MeetTarget,
    pub observed_on: &'a str,
    pub school_year: SchoolYear,
    pub performance_as_of: chrono::NaiveDate,
    pub capture_sha256: &'a str,
    pub index: &'a SchoolIndex,
    pub resolved: &'a mut HashMap<String, Option<SchoolId>>,
    pub stats: &'a mut ResultStats,
    pub accumulator: &'a mut Accumulator,
    pub failures: &'a mut Vec<String>,
}

impl Fold<'_> {
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
            performance_as_of: self.performance_as_of,
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

pub(super) fn absorb_document(
    fold: &mut Fold<'_>,
    path: &str,
    doc: EventDoc,
) -> Option<PublishedEvent> {
    let Some(capture_id) = doc.event_id() else {
        fold.failures
            .push(format!("{path}: the document carries no event id"));
        return None;
    };
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
    let event = match mint_event(fold, &doc, capture_id, &url) {
        Ok(event) => event,
        Err(error) => {
            fold.failures.push(format!("{path}: {error}"));
            return None;
        }
    };
    if let Err(error) = fold_rows(fold, &doc, &url, &event, &event.kind) {
        fold.failures.push(format!("{path}: {error}"));
        return None;
    }
    fold.stats.documents_read = fold.stats.documents_read.saturating_add(1);
    Some(event)
}

fn fold_rows(
    fold: &mut Fold<'_>,
    doc: &EventDoc,
    url: &str,
    event: &PublishedEvent,
    kind: &EventKind,
) -> crate::CrawlResult<()> {
    let source = SourceRef::new(SOURCE_ID, Some(url.to_string()));
    let mut evidence = Evidence::parsed(source.clone(), fold.observed_on);
    evidence.note = Some(format!(
        "capture sha256={}; {}: {} rows published for this event",
        fold.capture_sha256,
        event.event_key,
        doc.rows.len()
    ));
    let sport = sport_for(doc.is_xc(), &fold.target.name, &fold.target.date);
    let (context, mut writer) = fold.split_for(&source, &evidence, event, kind, sport);
    doc.rows
        .iter()
        .enumerate()
        .fold(Ok(()), |outcome, (row_index, row)| {
            let projected = record_row(&mut writer, &context, row, row_index).map(|_| ());
            outcome.and(projected)
        })
}

pub(super) fn absorb_summary(fold: &mut Fold<'_>, path: &str, body: &str) -> Option<Vec<u64>> {
    let url = event_summary_url(fold.target.athleticlive_meet_id);
    let events = match parse_event_summary(&url, body) {
        Ok(events) => events,
        Err(error) => {
            fold.failures.push(format!("{path}: {error}"));
            return None;
        }
    };
    let mut fetchable = Vec::new();
    match events
        .iter()
        .try_for_each(|event| absorb_listed(fold, &url, event, &mut fetchable))
    {
        Ok(()) => Some(fetchable),
        Err(error) => {
            fold.failures.push(format!("{url}: {error}"));
            None
        }
    }
}

fn absorb_listed(
    fold: &mut Fold<'_>,
    url: &str,
    event: &SummaryEvent,
    fetchable: &mut Vec<u64>,
) -> crate::CrawlResult<()> {
    fold.stats.events_listed = fold
        .stats
        .events_listed
        .checked_add(1)
        .ok_or_else(super::run::counter_error)?;
    let Some(event_id) = event.event_id() else {
        fold.failures
            .push(format!("{url}: listed event carries no native event id"));
        return Ok(());
    };
    if event.is_relay() {
        fold.stats.events_relay = fold
            .stats
            .events_relay
            .checked_add(1)
            .ok_or_else(super::run::counter_error)?;
        return Ok(());
    }
    if !matches!(
        crate::context::assess_performance_date(fold.performance_as_of, &fold.target.date),
        crate::context::PerformanceDateAssessment::Future
    ) && matches!(event.kind(), EventKind::Unmapped { .. })
    {
        fold.stats.events_unmapped = fold
            .stats
            .events_unmapped
            .checked_add(1)
            .ok_or_else(super::run::counter_error)?;
    }
    let requested = fetchable
        .len()
        .checked_add(1)
        .ok_or_else(super::run::counter_error)?;
    let resource = || crate::CrawlError::Resource {
        resource: "athleticlive listed events",
        requested,
        limit: 100_000,
    };
    if requested > 100_000 {
        return Err(resource());
    }
    fetchable.try_reserve(1).map_err(|_| resource())?;
    fetchable.push(event_id);
    Ok(())
}

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
        "capture sha256={}; {}: {} standings rows published for run {run_id}",
        fold.capture_sha256,
        event.event_key,
        rows.len()
    ));
    let sport = sport_for(false, &fold.target.name, &fold.target.date);
    let (context, mut writer) = fold.split_for(&source, &evidence, event, &event.kind, sport);
    let projected =
        rows.iter()
            .enumerate()
            .fold(Ok(()), |outcome, (row_index, (_run_row, row))| {
                let projected = record_standing(&mut writer, &context, row, row_index).map(|_| ());
                outcome.and(projected)
            });
    if let Err(error) = projected {
        fold.failures.push(format!("{path}: {error}"));
        return None;
    }
    fold.stats.standings_read = fold.stats.standings_read.saturating_add(1);
    Some(rows.len())
}
