use super::state::{Accumulator, Page};
use crate::tfrrs::parse::ParsedSection;
use crate::{CrawlError, CrawlResult};
use census_domain::model::{
    CanonicalEvent, EventId, EventIdentity, EventKind, EventSpecification, Evidence, Gender,
    MeetId, SourceEventLabel,
};

const EVENT_LIMIT: usize = 100_000;

pub(super) fn mint(
    meet: &MeetId,
    gender: Gender,
    section: &ParsedSection,
) -> CrawlResult<CanonicalEvent> {
    let kind = EventKind::from_source_label(&section.label);
    let specification = EventSpecification::from_published_label(&section.label, &kind)?;
    Ok(CanonicalEvent::new(
        EventIdentity {
            meet,
            kind,
            gender,
            division: None,
            round: None,
        },
        specification,
    )?)
}

pub(super) fn retain(
    accumulated: &mut Accumulator,
    minted: CanonicalEvent,
    page: Page<'_>,
    label: &str,
) -> CrawlResult<EventId> {
    let id = minted.id.clone();
    if !accumulated.events.contains_key(id.as_str()) {
        let requested = accumulated
            .events
            .len()
            .checked_add(1)
            .ok_or_else(|| resource("tfrrs events", usize::MAX))?;
        if requested > EVENT_LIMIT {
            return Err(resource("tfrrs events", requested));
        }
        accumulated
            .events
            .try_reserve(1)
            .map_err(|_| resource("tfrrs events", requested))?;
    }
    let event = accumulated
        .events
        .entry(id.as_str().into())
        .or_insert(minted);
    let label = SourceEventLabel {
        source: page.source.clone(),
        label: label.into(),
    };
    append_unique(&mut event.source_labels, label, "tfrrs event labels")?;
    append_unique(
        &mut event.evidence,
        Evidence::parsed(page.source.clone(), page.observed_on),
        "tfrrs event evidence",
    )?;
    Ok(id)
}

fn append_unique<T: PartialEq>(
    entries: &mut Vec<T>,
    value: T,
    name: &'static str,
) -> CrawlResult<()> {
    if entries.contains(&value) {
        return Ok(());
    }
    let requested = entries
        .len()
        .checked_add(1)
        .ok_or_else(|| resource(name, usize::MAX))?;
    if requested > EVENT_LIMIT {
        return Err(resource(name, requested));
    }
    entries
        .try_reserve(1)
        .map_err(|_| resource(name, requested))?;
    entries.push(value);
    Ok(())
}

fn resource(resource: &'static str, requested: usize) -> CrawlError {
    CrawlError::Resource {
        resource,
        requested,
        limit: EVENT_LIMIT,
    }
}
