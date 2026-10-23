use super::{ReportError, ReportResult};
use crate::export::ExportDataset;
use crate::grouping::{Admission, GroupingError};
use census_domain::model::{
    CanonicalEvent, EventDisposition, EventExclusionReason, EventId, EventKind,
    EventResolutionReason, MeetId, RecruitingCatalogV1, Sport,
};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};

mod selected;

const STAGE: &str = "recruiting event admission";

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct EventCounts {
    pub included: usize,
    pub excluded: BTreeMap<EventExclusionReason, usize>,
    pub unresolved: BTreeMap<EventResolutionReason, usize>,
}

#[derive(Debug, Serialize)]
pub struct AdmittedEvent<'a> {
    pub event: &'a EventId,
    pub meet: &'a MeetId,
    pub disposition: EventDisposition,
    #[serde(skip)]
    canonical: &'a CanonicalEvent,
    #[serde(skip)]
    sport: Sport,
}

pub struct EventAdmission<'a> {
    dataset: &'a ExportDataset,
    rows: Vec<AdmittedEvent<'a>>,
    counts: EventCounts,
    digest: String,
    semantics: census_domain::model::EventSemantics,
}

#[derive(Serialize)]
pub struct EventAudit<'a, 'd> {
    input_generation: &'a str,
    catalog_digest: &'a str,
    counts: &'a EventCounts,
    events: &'a [AdmittedEvent<'d>],
}

impl<'a> EventAdmission<'a> {
    pub fn of(dataset: &'a ExportDataset) -> ReportResult<Self> {
        let semantics = dataset.semantic_policy()?;
        let catalog = RecruitingCatalogV1::with_policy(semantics);
        let selection = selected::Selection::of(dataset)?;
        let meets = meet_sports(dataset)?;
        let mut rows = Vec::new();
        reserve::<AdmittedEvent<'_>>(dataset.events.len())?;
        rows.try_reserve(dataset.events.len()).map_err(allocation)?;
        let mut budget = Admission::new(STAGE);
        for event in &dataset.events {
            let sport = meets
                .get(&event.meet)
                .copied()
                .map_or(Sport::Unknown, |value| value);
            let disposition = selection.classify(&catalog, event, sport)?;
            budget.record(event)?;
            rows.push(AdmittedEvent {
                event: &event.id,
                meet: &event.meet,
                disposition,
                canonical: event,
                sport,
            });
        }
        rows.sort_by(|left, right| left.event.cmp(right.event));
        reject_duplicate_events(&rows)?;
        let counts = counts(&rows)?;
        let digest = catalog.digest().map_err(|source| ReportError::Invariant {
            detail: format!("encoding recruiting catalog identity: {source}"),
        })?;
        Ok(Self {
            dataset,
            rows,
            counts,
            digest,
            semantics,
        })
    }

    pub fn semantics(&self) -> census_domain::model::EventSemantics {
        self.semantics
    }

    pub fn dataset(&self) -> &'a ExportDataset {
        self.dataset
    }

    pub fn ensure_dataset(&self, dataset: &ExportDataset) -> ReportResult<()> {
        if !std::ptr::eq(self.dataset, dataset) {
            return Err(ReportError::Invariant {
                detail: "event admission belongs to another dataset".into(),
            });
        }
        Ok(())
    }

    pub fn rows(&self) -> &[AdmittedEvent<'a>] {
        &self.rows
    }

    pub fn counts(&self) -> &EventCounts {
        &self.counts
    }

    pub fn audit(&self) -> EventAudit<'_, 'a> {
        EventAudit {
            input_generation: &self.dataset.lineage.input_generation,
            catalog_digest: &self.digest,
            counts: &self.counts,
            events: &self.rows,
        }
    }

    pub fn disposition(&self, event: &EventId) -> Option<&EventDisposition> {
        self.rows
            .binary_search_by(|row| row.event.cmp(event))
            .ok()
            .and_then(|position| self.rows.get(position))
            .map(|row| &row.disposition)
    }

    pub fn event(&self, event: &EventId) -> Option<&'a CanonicalEvent> {
        self.rows
            .binary_search_by(|row| row.event.cmp(event))
            .ok()
            .and_then(|position| self.rows.get(position))
            .map(|row| row.canonical)
    }

    pub fn sport(&self, event: &EventId) -> Option<Sport> {
        self.rows
            .binary_search_by(|row| row.event.cmp(event))
            .ok()
            .and_then(|position| self.rows.get(position))
            .map(|row| row.sport)
    }

    pub fn included_kind(&self, event: &EventId) -> Option<&EventKind> {
        match self.disposition(event) {
            Some(EventDisposition::Included(kind)) => Some(kind),
            Some(EventDisposition::Excluded(_) | EventDisposition::Unresolved(_)) | None => None,
        }
    }
}

fn meet_sports(dataset: &ExportDataset) -> ReportResult<HashMap<&MeetId, Sport>> {
    let mut index = HashMap::new();
    reserve::<(&MeetId, Sport)>(dataset.meets.len())?;
    index.try_reserve(dataset.meets.len()).map_err(allocation)?;
    let mut budget = Admission::new(STAGE);
    for meet in &dataset.meets {
        budget.record(meet)?;
        let sport = super::population::meet_sport(meet);
        if index.insert(&meet.id, sport).is_some() {
            return Err(ReportError::Invariant {
                detail: format!("duplicate meet {}", meet.id),
            });
        }
    }
    Ok(index)
}

fn reject_duplicate_events(rows: &[AdmittedEvent<'_>]) -> ReportResult<()> {
    for pair in rows.windows(2) {
        if let [left, right] = pair {
            if left.event == right.event {
                return Err(ReportError::Invariant {
                    detail: format!("duplicate event {}", left.event),
                });
            }
        }
    }
    Ok(())
}

fn counts(rows: &[AdmittedEvent<'_>]) -> ReportResult<EventCounts> {
    let mut counts = EventCounts::default();
    for row in rows {
        let total = match &row.disposition {
            EventDisposition::Included(_) => &mut counts.included,
            EventDisposition::Excluded(reason) => counts.excluded.entry(*reason).or_default(),
            EventDisposition::Unresolved(reason) => counts.unresolved.entry(*reason).or_default(),
        };
        *total = total.checked_add(1).ok_or(GroupingError::Overflow {
            stage: STAGE,
            resource: "event disposition count",
        })?;
    }
    Ok(counts)
}

fn allocation(_: std::collections::TryReserveError) -> GroupingError {
    GroupingError::Allocation { stage: STAGE }
}

fn reserve<T>(rows: usize) -> ReportResult<()> {
    let bytes = rows
        .checked_mul(std::mem::size_of::<T>())
        .and_then(|bytes| bytes.checked_mul(4))
        .ok_or(GroupingError::Overflow {
            stage: STAGE,
            resource: "admission scratch bytes",
        })?;
    Admission::new(STAGE).charge(bytes, rows)?;
    Ok(())
}
