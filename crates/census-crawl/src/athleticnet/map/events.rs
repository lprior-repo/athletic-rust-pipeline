use super::{Accumulator, PerformanceInput};
use crate::{CrawlError, CrawlResult};
use census_domain::model::{
    CanonicalEvent, EventId, EventIdentity, EventKind, EventSpecification, Evidence,
    RetainedConflict, SourceEventLabel, SourceRef, SpecificationError,
};

const EVENT_LIMIT: usize = 100_000;

pub(in crate::athleticnet) fn ensure_event(
    accumulated: &mut Accumulator,
    source: &SourceRef,
    observed_on: &str,
    input: &PerformanceInput<'_>,
) -> CrawlResult<EventId> {
    let event = CanonicalEvent::new(identity(input), specification(input)?)?;
    retain_event(
        accumulated,
        event,
        input.labels,
        Evidence::parsed(source.clone(), observed_on),
    )
}

fn identity<'a>(input: &'a PerformanceInput<'_>) -> EventIdentity<'a> {
    EventIdentity {
        meet: &input.meet.id,
        kind: input.kind.clone(),
        gender: input.gender,
        division: input.division.as_deref(),
        round: input.round.as_deref(),
    }
}

fn specification(input: &PerformanceInput<'_>) -> CrawlResult<EventSpecification> {
    let mut labels = input.labels.iter().copied();
    let initial =
        EventSpecification::from_published_label(labels.next().map_or("", |v| v), input.kind)?;
    if input
        .labels
        .iter()
        .any(|label| alias_contradicts(input.kind, label))
    {
        return Ok(initial);
    }
    Ok(labels.try_fold(initial, |specification, label| {
        specification.merge(EventSpecification::from_published_label(label, input.kind)?)
    })?)
}

fn retain_event(
    accumulated: &mut Accumulator,
    event: CanonicalEvent,
    labels: &[&str],
    evidence: Evidence,
) -> CrawlResult<EventId> {
    let id = event.id.clone();
    reserve_events(accumulated, &id)?;
    let entry = accumulated
        .events
        .entry(id.as_str().into())
        .or_insert(event);
    labels
        .iter()
        .try_for_each(|label| retain_label(entry, label, &evidence.source))?;
    if !entry.evidence.contains(&evidence) {
        reserve(&mut entry.evidence, "athleticnet event evidence")?;
        entry.evidence.push(evidence);
    }
    admit_aliases(entry)?;
    Ok(id)
}

fn admit_aliases(event: &mut CanonicalEvent) -> CrawlResult<()> {
    let contradictory = event
        .source_labels
        .iter()
        .any(|label| alias_contradicts(&event.kind, &label.label));
    if contradictory {
        let conflict = RetainedConflict::new(
            "Event specification",
            event.id.as_str(),
            event.kind.stable_key().as_ref(),
            "Published event aliases contradict the event kind",
        );
        if !event.retained_conflicts.contains(&conflict) {
            reserve(&mut event.retained_conflicts, "athleticnet event conflicts")?;
            event.retained_conflicts.push(conflict);
        }
        return Err(SpecificationError::ConflictingSpecification.into());
    }
    if !event.retained_conflicts.is_empty() {
        return Err(SpecificationError::ConflictingSpecification.into());
    }
    Ok(())
}

fn alias_contradicts(kind: &EventKind, label: &str) -> bool {
    let published = EventKind::from_source_label(label);
    !matches!(published, EventKind::Unmapped { .. }) && published != *kind
}

fn retain_label(event: &mut CanonicalEvent, label: &str, source: &SourceRef) -> CrawlResult<()> {
    if event
        .source_labels
        .iter()
        .any(|known| known.label == label && known.source == *source)
    {
        return Ok(());
    }
    reserve(&mut event.source_labels, "athleticnet event labels")?;
    event.source_labels.push(SourceEventLabel {
        source: source.clone(),
        label: label.into(),
    });
    Ok(())
}

fn reserve_events(accumulated: &mut Accumulator, id: &EventId) -> CrawlResult<()> {
    if accumulated.events.contains_key(id.as_str()) {
        return Ok(());
    }
    let requested = accumulated
        .events
        .len()
        .checked_add(1)
        .ok_or_else(|| resource("athleticnet events", usize::MAX))?;
    if requested > EVENT_LIMIT {
        return Err(resource("athleticnet events", requested));
    }
    accumulated
        .events
        .try_reserve(1)
        .map_err(|_| resource("athleticnet events", requested))
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
