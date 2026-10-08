use super::super::map::{published_gender, unmapped, Mapper};
use super::super::parse::{class_token, event_label, round_label};
use super::super::wire::EventRow;
use crate::{CrawlError, CrawlResult};
use census_domain::model::{
    CanonicalEvent, CanonicalMeet, EventIdentity, EventKind, EventSpecification, SourceEventLabel,
};

const EVENT_LIMIT: usize = 100_000;

pub(super) fn mint(meet: &CanonicalMeet, row: &EventRow) -> CrawlResult<CanonicalEvent> {
    let kind = event_label(&row.event_name, &row.class_division)
        .map_or_else(|| unmapped(&row.event_name), EventKind::from_source_label);
    let specification = EventSpecification::from_published_label(&row.event_name, &kind)?;
    Ok(CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind,
            gender: published_gender(&row.gender),
            division: class_token(&row.class_division),
            round: round_label(row.round.as_deref()),
        },
        specification,
    )?)
}

pub(super) fn retain(
    mapper: &mut Mapper<'_>,
    event: &mut CanonicalEvent,
    label: &str,
    url: &str,
) -> CrawlResult<()> {
    reserve(&mut event.source_labels, "ihsa event labels")?;
    event.source_labels.push(SourceEventLabel {
        source: mapper.origin.source(url),
        label: label.into(),
    });
    reserve(&mut event.evidence, "ihsa event evidence")?;
    event.evidence.push(mapper.origin.evidence(url));
    reserve_events(mapper, event.id.as_str())?;
    let stored = mapper
        .accumulated
        .events
        .entry(event.id.as_str().into())
        .or_insert_with(|| event.clone());
    event.source_labels.iter().try_for_each(|label| {
        append_unique(&mut stored.source_labels, label, "ihsa event labels")
    })?;
    event.evidence.iter().try_for_each(|evidence| {
        append_unique(&mut stored.evidence, evidence, "ihsa event evidence")
    })
}

fn append_unique<T: PartialEq + Clone>(
    entries: &mut Vec<T>,
    value: &T,
    name: &'static str,
) -> CrawlResult<()> {
    if entries.contains(value) {
        return Ok(());
    }
    reserve(entries, name)?;
    entries.push(value.clone());
    Ok(())
}

fn reserve_events(mapper: &mut Mapper<'_>, id: &str) -> CrawlResult<()> {
    if mapper.accumulated.events.contains_key(id) {
        return Ok(());
    }
    let requested = mapper
        .accumulated
        .events
        .len()
        .checked_add(1)
        .ok_or_else(|| resource("ihsa events", usize::MAX))?;
    if requested > EVENT_LIMIT {
        return Err(resource("ihsa events", requested));
    }
    mapper
        .accumulated
        .events
        .try_reserve(1)
        .map_err(|_| resource("ihsa events", requested))
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
