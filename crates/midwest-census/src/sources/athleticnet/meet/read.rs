//! The readers behind the whole-meet walk: what a published jurisdiction, sport, grade, mark or
//! event type denotes, and where a relay's legs hang.

use super::super::parse::{metric_bare, parse_mark, published_date, published_token};
use super::wire::{EventDivisions, PublishedEvent, PublishedLeg};
use crate::sources::hytek::{parse_field_mark, parse_time};
use census_domain::model::{EventKind, Grade, Mark, Sport};
use census_domain::UsJurisdiction;
use std::collections::BTreeMap;

/// Resolve the legs of every relay, keyed by their parent result id, in published order.
pub(super) fn legs_by_result(legs: &[PublishedLeg]) -> BTreeMap<i64, Vec<&PublishedLeg>> {
    let mut grouped: BTreeMap<i64, Vec<&PublishedLeg>> = BTreeMap::new();
    for leg in legs {
        grouped.entry(leg.result_id).or_default().push(leg);
    }
    grouped
}

/// The jurisdiction a published `State` column denotes.
///
/// The column carries a USPS code (`"WI"`), so a spelled-out name, an empty column or the
/// out-of-scope `Overseas` region's absent code is refused — and the caller counts the refusal
/// rather than coercing the meet into a jurisdiction.
pub fn jurisdiction_of(published: Option<&str>) -> Option<UsJurisdiction> {
    UsJurisdiction::from_code(published?)
}

/// The `YYYY-MM-DD` date a published `MeetDate`/`EndDate` carries.
pub(super) fn meet_date(published: &str) -> Option<&str> {
    published_date(published)
}

/// The sport the meet document publishes: `sport2` is `"tfo"` outdoor, `"tfi"` indoor.
///
/// No indoor meet is captured in this lane, so `"tfi"` is the payload's own vocabulary read
/// literally and is otherwise unverified; anything else is the outdoor track this endpoint serves.
pub(super) fn sport_of(published: Option<&str>) -> Sport {
    match published.map(str::trim) {
        Some("tfi") => Sport::IndoorTrack,
        _ => Sport::OutdoorTrack,
    }
}

/// The grade a published token denotes: `9`..=`12`, and nothing else.
///
/// Athletic.net overloads the grade columns with placeholders — `99` on a relay or blurred
/// rankings row, `"-"` on a relay squad row — and neither is a grade, so both are refused here
/// rather than read as one.
pub fn grade_of(published: Option<&str>) -> Option<Grade> {
    Grade::new(published?.trim().parse::<u8>().ok()?)
}

/// The mark a published result token denotes, with the automatic-timing flag.
///
/// An event whose own label maps to a platform kind is read by that kind. An unmapped label is read
/// only from the metadata document's `Type`: a declared field event parses as a field mark and a
/// declared track event as a time. Without that type the row yields no mark (and the caller counts
/// it) — `"12.34"` on a shot put would otherwise read as 12.34 seconds.
pub(super) fn meet_mark(
    kind: &EventKind,
    published: &str,
    event_type: Option<&str>,
) -> Option<(Mark, bool)> {
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

/// The per-event metadata the third request publishes, keyed by the event id the results blocks
/// carry. It is the only place the payload declares whether an event is a track or a field event.
#[derive(Debug, Clone, Default)]
pub struct EventMetadata {
    by_event: BTreeMap<i64, PublishedEvent>,
}

impl EventMetadata {
    /// Index one metadata document by event id.
    pub fn new(divisions: &EventDivisions) -> Self {
        Self {
            by_event: divisions
                .events
                .iter()
                .map(|event| (event.id, event.clone()))
                .collect(),
        }
    }

    /// The published event type (`"T"` track, `"F"` field) for an event id.
    pub(super) fn event_type(&self, event_id: i64) -> Option<&str> {
        self.by_event.get(&event_id)?.event_type.as_deref()
    }

    /// Whether the payload declares the event a hurdle race.
    pub(super) fn is_hurdle(&self, event_id: i64) -> Option<bool> {
        Some(self.by_event.get(&event_id)?.is_hurdle)
    }

    /// The events the document lists.
    pub(super) fn len(&self) -> usize {
        self.by_event.len()
    }

    /// The events the document lists as field events.
    pub(super) fn field_events(&self) -> usize {
        self.by_event
            .values()
            .filter(|event| event.event_type.as_deref() == Some("F"))
            .count()
    }

    /// The events the document lists as hurdle races.
    pub(super) fn hurdles(&self) -> usize {
        self.by_event
            .values()
            .filter(|event| event.is_hurdle)
            .count()
    }
}

/// Whether the payload's declared type and the label's mapped kind agree, or `None` when the
/// payload declares no type (an absent `isHurdle` flag is not a disagreement with a flat race).
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

/// Whether a mapped kind is one of the hurdle races the payload can declare.
fn is_hurdle_kind(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Track100mHurdles
            | EventKind::Track110mHurdles
            | EventKind::Track300mHurdles
            | EventKind::Track400mHurdles
    )
}
