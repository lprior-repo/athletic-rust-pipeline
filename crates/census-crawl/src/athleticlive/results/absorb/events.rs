use super::{event_key, EventDoc, EventId, EventKind, Fold, Gender, PublishedEvent, SOURCE_ID};
use crate::athleticlive_athletes::gender_from_token;
use crate::hytek::round_marker;
use crate::{CrawlError, CrawlResult};
use census_domain::model::{
    CanonicalEvent, EventIdentity, EventSpecification, Evidence, SourceEventLabel, SourceRef,
    SpecificationError,
};

const EVENT_LIMIT: usize = 100_000;

pub(super) fn mint_event(
    fold: &mut Fold<'_>,
    doc: &EventDoc,
    capture_id: u64,
    url: &str,
) -> CrawlResult<PublishedEvent> {
    let kind = published_kind(doc)?;
    let gender = doc
        .gender_group
        .as_deref()
        .map(gender_from_token)
        .map_or(Gender::Unknown, |v| v);
    let round = doc
        .round_name
        .as_deref()
        .map(str::trim)
        .and_then(round_marker)
        .map(str::to_string);
    let event = CanonicalEvent::new(
        identity(&fold.meet.id, doc, &kind, gender, round.as_deref()),
        published_specification(doc, &kind)?,
    )?;
    let id = event.id.clone();
    retain_event(fold, doc, event, url)?;
    Ok(PublishedEvent {
        capture_id,
        id,
        kind,
        gender,
        round,
        run_id: doc.run_id().map(str::to_string),
        event_key: event_key(capture_id),
    })
}

fn identity<'a>(
    meet: &'a census_domain::model::MeetId,
    doc: &'a EventDoc,
    kind: &EventKind,
    gender: Gender,
    round: Option<&'a str>,
) -> EventIdentity<'a> {
    EventIdentity {
        meet,
        kind: kind.clone(),
        gender,
        division: doc.division_name(),
        round,
    }
}

fn published_labels(doc: &EventDoc) -> impl Iterator<Item = &str> {
    [
        doc.name.as_deref(),
        doc.short_name.as_deref(),
        doc.abbrev.as_deref(),
        doc.unit.as_deref(),
    ]
    .into_iter()
    .flatten()
}

fn published_kind(doc: &EventDoc) -> CrawlResult<EventKind> {
    if doc.is_xc() {
        return Ok(EventKind::CrossCountry);
    }
    let known = published_labels(doc)
        .map(EventKind::from_source_label)
        .filter(|kind| !matches!(kind, EventKind::Unmapped { .. }))
        .try_fold(None, |previous, kind| {
            if previous.as_ref().is_some_and(|previous| previous != &kind) {
                return Err(SpecificationError::ConflictingSpecification);
            }
            Ok(Some(kind))
        })?;
    Ok(known.map_or_else(|| doc.kind(), core::convert::identity))
}

fn published_specification(doc: &EventDoc, kind: &EventKind) -> CrawlResult<EventSpecification> {
    let mut labels = published_labels(doc);
    let initial = EventSpecification::from_published_label(labels.next().map_or("", |v| v), kind)?;
    labels.try_fold(initial, |specification, label| {
        Ok(specification.merge(EventSpecification::from_published_label(label, kind)?)?)
    })
}

fn retain_event(
    fold: &mut Fold<'_>,
    doc: &EventDoc,
    event: CanonicalEvent,
    url: &str,
) -> CrawlResult<()> {
    let source = SourceRef::new(SOURCE_ID, Some(url.into()));
    let mut evidence = Evidence::parsed(source.clone(), fold.observed_on);
    evidence.note = Some(format!("capture sha256={}", fold.capture_sha256));
    reserve_events(fold, &event.id)?;
    let entry = fold
        .accumulator
        .events
        .entry(event.id.as_str().into())
        .or_insert(event);
    published_labels(doc).try_for_each(|label| retain_label(entry, label, &source))?;
    if !entry.evidence.contains(&evidence) {
        reserve(&mut entry.evidence, "athleticlive event evidence")?;
        entry.evidence.push(evidence);
    }
    Ok(())
}

fn retain_label(event: &mut CanonicalEvent, label: &str, source: &SourceRef) -> CrawlResult<()> {
    if event
        .source_labels
        .iter()
        .any(|known| known.label == label && known.source == *source)
    {
        return Ok(());
    }
    reserve(&mut event.source_labels, "athleticlive event labels")?;
    event.source_labels.push(SourceEventLabel {
        source: source.clone(),
        label: label.into(),
    });
    Ok(())
}

fn reserve_events(fold: &mut Fold<'_>, id: &EventId) -> CrawlResult<()> {
    if fold.accumulator.events.contains_key(id.as_str()) {
        return Ok(());
    }
    let requested = fold
        .accumulator
        .events
        .len()
        .checked_add(1)
        .ok_or_else(|| resource("athleticlive events", usize::MAX))?;
    if requested > EVENT_LIMIT {
        return Err(resource("athleticlive events", requested));
    }
    fold.accumulator
        .events
        .try_reserve(1)
        .map_err(|_| resource("athleticlive events", requested))
}

fn reserve<T>(entries: &mut Vec<T>, name: &'static str) -> CrawlResult<()> {
    let requested = entries
        .len()
        .checked_add(1)
        .ok_or_else(|| resource(name, usize::MAX))?;
    if requested > EVENT_LIMIT {
        return Err(resource(name, requested));
    }
    entries
        .try_reserve(1)
        .map_err(|_| resource(name, requested))
}

fn resource(resource: &'static str, requested: usize) -> CrawlError {
    CrawlError::Resource {
        resource,
        requested,
        limit: EVENT_LIMIT,
    }
}
