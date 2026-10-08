use super::super::parse::{metric_bare, parse_mark, published_date, published_token};
use super::wire::{EventDivisions, PublishedEvent, PublishedLeg};
use crate::hytek::{parse_field_mark, parse_time};
use census_domain::model::{EventKind, Grade, Mark, Sport};
use census_domain::UsJurisdiction;
use std::collections::BTreeMap;

pub(super) fn legs_by_result(legs: &[PublishedLeg]) -> BTreeMap<i64, Vec<&PublishedLeg>> {
    let mut grouped: BTreeMap<i64, Vec<&PublishedLeg>> = BTreeMap::new();
    for leg in legs {
        grouped.entry(leg.result_id).or_default().push(leg);
    }
    grouped
}

pub fn jurisdiction_of(published: Option<&str>) -> Option<UsJurisdiction> {
    UsJurisdiction::from_code(published?)
}

pub(super) fn meet_date(published: &str) -> Option<&str> {
    published_date(published)
}

pub(super) fn sport_of(published: Option<&str>) -> Option<Sport> {
    match published.map(str::trim) {
        Some("tfi") => Some(Sport::IndoorTrack),
        Some("tfo") => Some(Sport::OutdoorTrack),
        _ => None,
    }
}

pub fn grade_of(published: Option<&str>) -> Option<Grade> {
    Grade::new(published?.trim().parse::<u8>().ok()?)
}

pub(super) fn meet_mark(
    kind: &EventKind,
    published: &str,
    event_type: Option<&str>,
) -> Option<(Mark, bool)> {
    if crate::result_status::invalid_token(published).is_some() {
        return Some((Mark::Raw(published.to_string()), false));
    }
    if !matches!(kind, EventKind::Unmapped { .. }) {
        return parse_mark(kind, published);
    }
    let (token, auto) = published_token(published)?;
    let mark = match event_type.map(str::trim) {
        Some("F") => parse_field_mark(metric_bare(token))?,
        Some("T") => Mark::TimeSeconds(parse_time(token)?),
        _ => return None,
    };
    Some((mark, auto))
}

#[derive(Debug, Clone, Default)]
pub struct EventMetadata {
    by_event: BTreeMap<i64, PublishedEvent>,
}

impl EventMetadata {
    pub fn new(divisions: &EventDivisions) -> Self {
        Self {
            by_event: divisions
                .events
                .iter()
                .map(|event| (event.id, event.clone()))
                .collect(),
        }
    }

    pub(super) fn event_type(&self, event_id: i64) -> Option<&str> {
        self.by_event.get(&event_id)?.event_type.as_deref()
    }

    pub(super) fn is_hurdle(&self, event_id: i64) -> Option<bool> {
        Some(self.by_event.get(&event_id)?.is_hurdle)
    }

    pub(super) fn len(&self) -> usize {
        self.by_event.len()
    }

    pub(super) fn field_events(&self) -> usize {
        self.by_event
            .values()
            .filter(|event| event.event_type.as_deref() == Some("F"))
            .count()
    }

    pub(super) fn hurdles(&self) -> usize {
        self.by_event
            .values()
            .filter(|event| event.is_hurdle)
            .count()
    }
}

pub(super) fn event_type_agrees(
    kind: &EventKind,
    metadata: &EventMetadata,
    event_id: i64,
) -> Option<bool> {
    let declared_field = match metadata.event_type(event_id)? {
        "F" => true,
        "T" => false,
        _ => return None,
    };
    if !matches!(kind, EventKind::Unmapped { .. }) && declared_field != kind.is_field() {
        return Some(false);
    }
    if metadata.is_hurdle(event_id) == Some(true) && !is_hurdle_kind(kind) {
        return Some(false);
    }
    Some(true)
}

fn is_hurdle_kind(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Track100mHurdles
            | EventKind::Track110mHurdles
            | EventKind::Track300mHurdles
            | EventKind::Track400mHurdles
    )
}
