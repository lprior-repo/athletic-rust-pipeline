use super::{owned, MeetContext, OwnedPerformance, RowWriter};
use crate::{CrawlError, CrawlResult};
use census_domain::model::{
    CanonicalEvent, EventId, EventIdentity, EventKind, EventSpecification, Evidence,
    SourceEventLabel, SpecificationError,
};

const EVENT_LIMIT: usize = 100_000;

pub(super) fn record_event(
    writer: &mut RowWriter<'_>,
    context: &MeetContext<'_>,
    row: &OwnedPerformance,
    evidence: &Evidence,
) -> CrawlResult<EventId> {
    let event = CanonicalEvent::new(identity(context, row), specification(row)?)?;
    let id = event.id.clone();
    reserve_events(writer, &id)?;
    let entry = match writer.accumulated.events.entry(id.as_str().into()) {
        std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
        std::collections::hash_map::Entry::Vacant(entry) => {
            writer.stats.events = writer.stats.events.saturating_add(1);
            entry.insert(event)
        }
    };
    retain(entry, row, evidence)?;
    Ok(id)
}

fn identity<'a>(context: &'a MeetContext<'_>, row: &'a OwnedPerformance) -> EventIdentity<'a> {
    EventIdentity {
        meet: &context.meet.id,
        kind: row.event_kind.clone(),
        gender: row.gender,
        division: owned::text(row, "divisionName"),
        round: owned::text(row, "roundName"),
    }
}

fn labels(row: &OwnedPerformance) -> impl Iterator<Item = &str> {
    [owned::text(row, "eventName"), owned::text(row, "eventCode")]
        .into_iter()
        .flatten()
}

fn specification(row: &OwnedPerformance) -> CrawlResult<EventSpecification> {
    let mut labels = labels(row);
    let initial =
        EventSpecification::from_published_label(labels.next().map_or("", |v| v), &row.event_kind)?;
    labels.try_fold(initial, |specification, label| {
        let kind = EventKind::from_source_label(label);
        if !matches!(kind, EventKind::Unmapped { .. }) && kind != row.event_kind {
            return Err(SpecificationError::ConflictingSpecification.into());
        }
        Ok(
            specification.merge(EventSpecification::from_published_label(
                label,
                &row.event_kind,
            )?)?,
        )
    })
}

fn retain(
    event: &mut CanonicalEvent,
    row: &OwnedPerformance,
    evidence: &Evidence,
) -> CrawlResult<()> {
    labels(row).try_for_each(|label| retain_label(event, label, evidence))?;
    if !event.evidence.contains(evidence) {
        reserve(&mut event.evidence, 1, "milesplit event evidence")?;
        event.evidence.push(evidence.clone());
    }
    Ok(())
}

fn retain_label(event: &mut CanonicalEvent, label: &str, evidence: &Evidence) -> CrawlResult<()> {
    if event
        .source_labels
        .iter()
        .any(|known| known.label == label && known.source == evidence.source)
    {
        return Ok(());
    }
    reserve(&mut event.source_labels, 1, "milesplit event labels")?;
    event.source_labels.push(SourceEventLabel {
        source: evidence.source.clone(),
        label: label.into(),
    });
    Ok(())
}

fn reserve_events(writer: &mut RowWriter<'_>, id: &EventId) -> CrawlResult<()> {
    if writer.accumulated.events.contains_key(id.as_str()) {
        return Ok(());
    }
    let requested = writer
        .accumulated
        .events
        .len()
        .checked_add(1)
        .ok_or_else(|| resource("milesplit events", usize::MAX))?;
    if requested > EVENT_LIMIT {
        return Err(resource("milesplit events", requested));
    }
    writer
        .accumulated
        .events
        .try_reserve(1)
        .map_err(|_| resource("milesplit events", requested))
}

fn reserve<T>(entries: &mut Vec<T>, additional: usize, name: &'static str) -> CrawlResult<()> {
    let requested = entries
        .len()
        .checked_add(additional)
        .ok_or_else(|| resource(name, usize::MAX))?;
    if requested > EVENT_LIMIT {
        return Err(resource(name, requested));
    }
    entries
        .try_reserve(additional)
        .map_err(|_| resource(name, requested))
}

fn resource(resource: &'static str, requested: usize) -> CrawlError {
    CrawlError::Resource {
        resource,
        requested,
        limit: EVENT_LIMIT,
    }
}
