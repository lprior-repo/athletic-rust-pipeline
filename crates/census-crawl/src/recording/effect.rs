use super::{budget, once, Recorded, RecordedBatch, RecordedEffect, RecordedJournal};
use crate::{CrawlError, CrawlResult};
use std::ops::Range;

pub(super) fn pending(
    recorded: &Recorded,
    rows: &[RecordedBatch],
    journal: &[RecordedJournal],
) -> CrawlResult<RecordedEffect> {
    let rows = recorded.rows.len()..budget::add(recorded.rows.len(), rows.len())?;
    let journal = recorded.journal.len()..budget::add(recorded.journal.len(), journal.len())?;
    let witness = journal
        .end
        .checked_sub(1)
        .ok_or_else(|| invariant("conditional effect requires a witness"))?;
    Ok(RecordedEffect {
        rows,
        journal,
        witness,
    })
}

pub(super) fn validate(
    effect: &RecordedEffect,
    recorded: &Recorded,
    previous: &RecordedEffect,
) -> CrawlResult<()> {
    validate_range(&effect.rows, recorded.rows.len(), previous.rows.end)?;
    validate_range(
        &effect.journal,
        recorded.journal.len(),
        previous.journal.end,
    )?;
    if effect.journal.is_empty() || effect.journal.end.checked_sub(1) != Some(effect.witness) {
        return Err(invariant(
            "conditional effect witness is outside its journal range",
        ));
    }
    let entry = recorded
        .journal
        .get(effect.witness)
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
