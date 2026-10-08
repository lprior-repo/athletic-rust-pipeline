use super::{CanonicalMeet, EventId, Evidence, ParsedEvent, RowWriter};
use crate::{CrawlError, CrawlResult};
use census_domain::model::{CanonicalEvent, EventIdentity, EventSpecification, SourceEventLabel};

const EVENT_LIMIT: usize = 100_000;

pub(super) fn record_event(
    writer: &mut RowWriter<'_>,
    meet: &CanonicalMeet,
    published: &ParsedEvent,
    evidence: &Evidence,
) -> CrawlResult<EventId> {
    let event = CanonicalEvent::new(
        identity(meet, published),
        EventSpecification::from_published_label(&published.label, &published.kind)?,
    )?;
    let id = event.id.clone();
    reserve_events(writer, &id)?;
    writer.stats.events = writer.stats.events.saturating_add(1);
    let stored = writer
        .accumulator
        .events
        .entry(id.as_str().into())
        .or_insert(event);
    retain_label(stored, &published.label, evidence)?;
    if !stored.evidence.contains(evidence) {
        reserve(&mut stored.evidence, "wiaa event evidence")?;
        stored.evidence.push(evidence.clone());
    }
    Ok(id)
}

fn identity<'a>(meet: &'a CanonicalMeet, published: &'a ParsedEvent) -> EventIdentity<'a> {
    EventIdentity {
        meet: &meet.id,
        kind: published.kind.clone(),
        gender: published.gender,
        division: published.division.as_deref(),
        round: published.round.as_deref(),
    }
}

fn retain_label(event: &mut CanonicalEvent, label: &str, evidence: &Evidence) -> CrawlResult<()> {
    if event
        .source_labels
        .iter()
        .any(|known| known.label == label && known.source == evidence.source)
    {
        return Ok(());
    }
    reserve(&mut event.source_labels, "wiaa event labels")?;
    event.source_labels.push(SourceEventLabel {
        source: evidence.source.clone(),
        label: label.into(),
    });
    Ok(())
}

fn reserve_events(writer: &mut RowWriter<'_>, id: &EventId) -> CrawlResult<()> {
    if writer.accumulator.events.contains_key(id.as_str()) {
        return Ok(());
    }
    let requested = writer
        .accumulator
        .events
        .len()
        .checked_add(1)
        .ok_or_else(|| resource("wiaa events", usize::MAX))?;
    if requested > EVENT_LIMIT {
        return Err(resource("wiaa events", requested));
    }
    writer
        .accumulator
        .events
        .try_reserve(1)
        .map_err(|_| resource("wiaa events", requested))
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
