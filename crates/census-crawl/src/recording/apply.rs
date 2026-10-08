use super::{
    budget, effect::invariant, once, Recorded, RecordedBatch, RecordedEffect, RecordedJournal,
};
use crate::CrawlResult;
use census_store::{Application, ConditionalBatch, Store, StoreBatch, MAX_OPERATION_BYTES};

pub struct PreparedRecorded {
    recorded: Recorded,
    operation: String,
    digest: String,
}

impl PreparedRecorded {
    pub fn operation(&self) -> &str {
        &self.operation
    }

    pub fn apply(&self, store: &Store) -> CrawlResult<Application> {
        apply(&self.recorded, store, &self.operation, &self.digest)
    }
}

impl Recorded {
    pub fn prepare(self, prefix: &str) -> CrawlResult<PreparedRecorded> {
        let (operation, digest) = identity(&self, prefix)?;
        Ok(PreparedRecorded {
            recorded: self,
            operation,
            digest,
        })
    }

    pub fn apply(&self, store: &Store, prefix: &str) -> CrawlResult<Application> {
        let (operation, digest) = identity(self, prefix)?;
        apply(self, store, &operation, &digest)
    }
}

fn identity(recorded: &Recorded, prefix: &str) -> CrawlResult<(String, String)> {
    validate(recorded)?;
    let digest = digest(recorded)?;
    Ok((operation_key(prefix, &digest)?, digest))
}

fn apply(
    recorded: &Recorded,
    store: &Store,
    operation: &str,
    digest: &str,
) -> CrawlResult<Application> {
    let mut plain = store.write_batch();
    let mut effects = Vec::new();
    effects
        .try_reserve_exact(recorded.effects.len())
        .map_err(|_| {
            budget::resource(
                "conditional staging allocation",
                recorded.effects.len(),
                super::MAX_RECORDED_WORK,
            )
        })?;
    let cursor = recorded
        .effects
        .iter()
        .try_fold(empty_extent(), |cursor, effect| {
            stage_gap(&mut plain, recorded, &cursor, effect)?;
            if let Some(batch) = stage_effect(store, recorded, effect)? {
                effects.push(batch);
            }
            Ok::<_, crate::CrawlError>(effect.clone())
        })?;
    stage_gap(&mut plain, recorded, &cursor, &end_extent(recorded))?;
    Ok(plain.commit_once_with_effects(operation, digest, effects)?)
}

fn validate(recorded: &Recorded) -> CrawlResult<()> {
    let usage = budget::measure(&recorded.rows, &recorded.journal)?;
    let storage = budget::multiply(recorded.effects.len(), budget::EFFECT_STORAGE)?;
    usage.admitted(super::RecordingUsage {
        retained_bytes: storage,
        work: recorded.effects.len(),
    })?;
    recorded
        .effects
        .iter()
        .try_fold(empty_extent(), |previous, effect| {
            effect.validate(recorded, &previous)?;
            Ok::<_, crate::CrawlError>(effect.clone())
        })?;
    Ok(())
}

fn stage_effect<'store, 'recorded>(
    store: &'store Store,
    recorded: &'recorded Recorded,
    effect: &RecordedEffect,
) -> CrawlResult<Option<ConditionalBatch<'store, 'recorded>>> {
    let entry = recorded
        .journal
        .get(effect.witness)
        .ok_or_else(|| invariant("conditional effect witness is missing"))?;
    let digest = entry
        .payload
        .get("digest")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| invariant("conditional effect witness has no digest"))?;
    let identity = once::Effect::new(&entry.key, digest)?;
    if identity.committed(store)? {
        return Ok(None);
    }
    let rows = recorded
        .rows
        .get(effect.rows.clone())
        .ok_or_else(|| invariant("conditional rows are missing"))?;
    let journal = recorded
        .journal
        .get(effect.journal.clone())
        .ok_or_else(|| invariant("conditional journal is missing"))?;
    let mut batch = store.write_batch();
    stage(&mut batch, rows, journal)?;
    Ok(Some(
        batch.conditional(identity.operation(), identity.digest())?,
    ))
}

fn stage_gap(
    batch: &mut StoreBatch<'_>,
    recorded: &Recorded,
    previous: &RecordedEffect,
    next: &RecordedEffect,
) -> CrawlResult<()> {
    let rows = recorded
        .rows
        .get(previous.rows.end..next.rows.start)
        .ok_or_else(|| invariant("plain row range is invalid"))?;
    let journal = recorded
        .journal
        .get(previous.journal.end..next.journal.start)
        .ok_or_else(|| invariant("plain journal range is invalid"))?;
    stage(batch, rows, journal)
}

fn stage(
    batch: &mut StoreBatch<'_>,
    rows: &[RecordedBatch],
    journal: &[RecordedJournal],
) -> CrawlResult<()> {
    rows.iter()
        .try_for_each(|row| batch.record_many(row.table, &row.rows).map(|_| ()))?;
    journal.iter().try_for_each(|entry| {
        batch
            .journal_done(&entry.phase, &entry.key, &entry.payload)
            .map(|_| ())
    })?;
    Ok(())
}

fn empty_extent() -> RecordedEffect {
    RecordedEffect {
        rows: 0..0,
        journal: 0..0,
        witness: 0,
    }
}

fn end_extent(recorded: &Recorded) -> RecordedEffect {
    RecordedEffect {
        rows: recorded.rows.len()..recorded.rows.len(),
        journal: recorded.journal.len()..recorded.journal.len(),
        witness: 0,
    }
}

pub(crate) fn digest<T: serde::Serialize + ?Sized>(value: &T) -> CrawlResult<String> {
    census_domain::model::serialized_digest(value)
        .map_err(|error| invariant(&format!("source effect digest: {error}")))
}

pub(crate) fn operation_key(prefix: &str, digest: &str) -> CrawlResult<String> {
    let length = budget::add(budget::add(prefix.len(), 1)?, digest.len())?;
    budget::check("source effect operation bytes", length, MAX_OPERATION_BYTES)?;
    if prefix.is_empty() {
        return Err(invariant("source effect operation prefix is empty"));
    }
    let mut operation = String::new();
    operation.try_reserve_exact(length).map_err(|_| {
        budget::resource(
            "source effect operation allocation",
            length,
            MAX_OPERATION_BYTES,
        )
    })?;
    operation.push_str(prefix);
    operation.push(':');
    operation.push_str(digest);
    once::Effect::new(&operation, digest)?;
    Ok(operation)
}
