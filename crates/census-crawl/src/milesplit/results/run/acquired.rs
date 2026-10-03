use super::super::super::map::OwnedResultSet;
use super::super::super::owned::{OwnedMeetOutcome, OwnedMeetVerdict};
use std::collections::HashMap;

#[derive(Debug)]
pub(in crate::milesplit::results) struct AcquiredMeet {
    pub(super) outcome: OwnedMeetOutcome,
    result_sets: HashMap<u64, Vec<usize>>,
}

impl AcquiredMeet {
    pub(super) fn new(outcome: OwnedMeetOutcome) -> Self {
        let result_sets = match &outcome.verdict {
            OwnedMeetVerdict::Parsed(page) => page.rows.iter().enumerate().fold(
                HashMap::<u64, Vec<usize>>::new(),
                |mut sets, (index, row)| {
                    sets.entry(row.result_set_id).or_default().push(index);
                    sets
                },
            ),
            _ => HashMap::new(),
        };
        Self {
            outcome,
            result_sets,
        }
    }

    pub(super) fn result_set(&self, id: &str) -> Option<OwnedResultSet<'_>> {
        let OwnedMeetVerdict::Parsed(page) = &self.outcome.verdict else {
            return None;
        };
        let id = id.parse::<u64>().ok()?;
        let indices = self.result_sets.get(&id).map_or(&[][..], Vec::as_slice);
        Some(OwnedResultSet {
            capture: &self.outcome.capture,
            page,
            indices,
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
