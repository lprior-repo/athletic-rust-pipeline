use crate::athleticnet::map::{Accumulator, PerformanceInput};
use crate::{CrawlError, CrawlResult};
use census_domain::model::{EventId, RetainedConflict, SourceRef, SpecificationError};

const METADATA_CONFLICT: &str = "Published event metadata";
const CONFLICT_LIMIT: usize = 100_000;

#[derive(Clone, Copy)]
pub(in crate::athleticnet::meet) struct MetadataConflict<'a> {
    pub(super) event_id: i64,
    pub(super) declared_type: Option<&'a str>,
    pub(super) is_hurdle: Option<bool>,
}

pub(in crate::athleticnet::meet) enum ProjectionOutcome {
    Admitted(EventId),
    Withheld(EventId, RetainedConflict),
}

pub(in crate::athleticnet::meet) fn project_event(
    accumulated: &mut Accumulator,
    source: &SourceRef,
    observed_on: &str,
    input: &PerformanceInput<'_>,
    metadata: Option<MetadataConflict<'_>>,
) -> CrawlResult<ProjectionOutcome> {
    let projected =
        crate::athleticnet::map::events::ensure_event(accumulated, source, observed_on, input);
    let event_id = match projected {
        Ok(event) => event,
        Err(error @ CrawlError::Specification(SpecificationError::ConflictingSpecification)) => {
            let Some(event) = retained_metadata_event(accumulated, source, input) else {
                return Err(error);
            };
            event
        }
        Err(error) => return Err(error),
    };
    let event = accumulated
        .events
        .get_mut(event_id.as_str())
        .ok_or_else(|| CrawlError::Invariant {
            detail: format!("projected event {event_id} is absent"),
        })?;
    if let Some(metadata) = metadata {
        let conflict = RetainedConflict::new(
            METADATA_CONFLICT,
            event_id.as_str(),
            event.kind.stable_key().as_ref(),
            format!(
                "EventId {}; Type {:?}; isHurdle {:?}; published aliases {:?}; source {:?}",
                metadata.event_id, metadata.declared_type, metadata.is_hurdle, input.labels, source,
            ),
        );
        if !event.retained_conflicts.contains(&conflict) {
            reserve_conflict(&mut event.retained_conflicts)?;
            event.retained_conflicts.push(conflict.clone());
        }
        return Ok(ProjectionOutcome::Withheld(event_id, conflict));
    }
    match event.retained_conflicts.first() {
        Some(conflict) => Ok(ProjectionOutcome::Withheld(event_id, conflict.clone())),
        None => Ok(ProjectionOutcome::Admitted(event_id)),
    }
}

fn retained_metadata_event(
    accumulated: &Accumulator,
    source: &SourceRef,
    input: &PerformanceInput<'_>,
) -> Option<EventId> {
    let mut candidates = accumulated.events.values().filter(|event| {
        event.meet == input.meet.id
            && event.kind == *input.kind
            && event.gender == input.gender
            && event.division == input.division
            && event.round == input.round
            && !event.retained_conflicts.is_empty()
            && event
                .retained_conflicts
                .iter()
                .all(|conflict| conflict.family == METADATA_CONFLICT)
            && input.labels.iter().all(|label| {
                event
                    .source_labels
                    .iter()
                    .any(|retained| retained.label == *label && retained.source == *source)
            })
    });
    let candidate = candidates.next()?;
    candidates.next().is_none().then(|| candidate.id.clone())
}

fn reserve_conflict(conflicts: &mut Vec<RetainedConflict>) -> CrawlResult<()> {
    let requested = conflicts.len().checked_add(1).ok_or(CrawlError::Resource {
        resource: "athleticnet metadata conflicts",
        requested: usize::MAX,
        limit: CONFLICT_LIMIT,
    })?;
    if requested > CONFLICT_LIMIT {
        return Err(CrawlError::Resource {
            resource: "athleticnet metadata conflicts",
            requested,
            limit: CONFLICT_LIMIT,
        });
    }
    conflicts.try_reserve(1).map_err(|_| CrawlError::Resource {
        resource: "athleticnet metadata conflicts",
        requested,
        limit: CONFLICT_LIMIT,
    })
}
