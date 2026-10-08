use super::super::super::map::OwnedResultSet;
use super::super::super::owned::{OwnedMeetOutcome, OwnedMeetVerdict, MAX_OWNED_ROWS};
use super::super::budget;
use crate::{CrawlError, CrawlResult};
use std::collections::HashMap;

#[derive(Debug)]
pub(in crate::milesplit::results) struct AcquiredMeet {
    pub(super) outcome: OwnedMeetOutcome,
    result_sets: HashMap<u64, Vec<usize>>,
}

impl AcquiredMeet {
    pub(super) fn new(outcome: OwnedMeetOutcome) -> CrawlResult<Self> {
        let mut result_sets = HashMap::new();
        if let OwnedMeetVerdict::Parsed(page) = &outcome.verdict {
            if page.rows.len() > MAX_OWNED_ROWS {
                return Err(CrawlError::Invariant {
                    detail: "owned projection row admission exceeded".into(),
                });
            }
            result_sets
                .try_reserve(page.rows.len())
                .map_err(budget::reserve)?;
            page.rows.iter().enumerate().try_for_each(|(index, row)| {
                let indices = result_sets
                    .entry(row.result_set_id)
                    .or_insert_with(Vec::new);
                indices.try_reserve(1).map_err(budget::reserve)?;
                indices.push(index);
                Ok::<_, CrawlError>(())
            })?;
        }
        Ok(Self {
            outcome,
            result_sets,
        })
    }

    pub(super) fn release_body(&mut self) {
        self.outcome.capture.body = Vec::new();
    }

    pub(super) fn result_set(
        &self,
        id: &str,
        performance_as_of: chrono::NaiveDate,
    ) -> Option<OwnedResultSet<'_>> {
        let OwnedMeetVerdict::Parsed(page) = &self.outcome.verdict else {
            return None;
        };
        let id = id.parse::<u64>().ok()?;
        Some(OwnedResultSet {
            capture: &self.outcome.capture,
            page,
            indices: self.result_sets.get(&id).map_or(&[], Vec::as_slice),
            performance_as_of,
        })
    }

    pub(super) fn failure(&self) -> Option<String> {
        let url = &self.outcome.capture.url;
        match &self.outcome.verdict {
            OwnedMeetVerdict::Parsed(page) if !page.individual_parse_complete() => Some(format!(
                "{url}: partial owned-meet parse; rejected individual rows retained with exact locators",
            )),
            OwnedMeetVerdict::Parsed(_) => None,
            verdict => Some(format!("{url}: {verdict:?}")),
        }
    }
}
