use super::{budget, once, Recorded, RecordedBatch, RecordedJournal};
use crate::{CrawlError, CrawlResult};
use serde::{Deserialize, Serialize};
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordedEffect {
    pub rows: Range<usize>,
    pub journal: Range<usize>,
    pub witness: usize,
}

impl RecordedEffect {
    pub(super) fn pending(
        recorded: &Recorded,
        rows: &[RecordedBatch],
        journal: &[RecordedJournal],
    ) -> CrawlResult<Self> {
        let rows = recorded.rows.len()..budget::add(recorded.rows.len(), rows.len())?;
        let journal = recorded.journal.len()..budget::add(recorded.journal.len(), journal.len())?;
        let witness = journal
            .end
            .checked_sub(1)
            .ok_or_else(|| invariant("conditional effect requires a witness"))?;
        Ok(Self {
            rows,
            journal,
            witness,
        })
    }

    pub(super) fn validate(&self, recorded: &Recorded, previous: &Self) -> CrawlResult<()> {
        validate_range(&self.rows, recorded.rows.len(), previous.rows.end)?;
        validate_range(&self.journal, recorded.journal.len(), previous.journal.end)?;
        if self.journal.is_empty() || self.journal.end.checked_sub(1) != Some(self.witness) {
            return Err(invariant(
                "conditional effect witness is outside its journal range",
            ));
        }
        let entry = recorded
            .journal
            .get(self.witness)
            .ok_or_else(|| invariant("conditional effect witness is missing"))?;
        if entry.phase != once::PHASE {
            return Err(invariant("conditional effect witness uses another phase"));
        }
        let digest = entry
            .payload
            .get("digest")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| invariant("conditional effect witness has no digest"))?;
        once::Effect::new(&entry.key, digest)?;
        Ok(())
    }
}

fn validate_range(range: &Range<usize>, length: usize, previous_end: usize) -> CrawlResult<()> {
    if range.start < previous_end || range.end < range.start || range.end > length {
        return Err(invariant(
            "conditional effect ranges overlap or exceed their recording",
        ));
    }
    Ok(())
}

pub(super) fn invariant(detail: &str) -> CrawlError {
    CrawlError::Invariant {
        detail: detail.to_owned(),
    }
}
