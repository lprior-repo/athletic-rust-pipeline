use super::{RecordedBatch, RecordedJournal};
use crate::{CrawlError, CrawlResult};
use std::mem::size_of;

mod serialized;
mod values;
pub(crate) use serialized::{preflight, preflight_rows};

pub const MAX_RECORDED_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_RECORDED_WORK: usize = 100_000;
pub(in crate::recording) const EFFECT_STORAGE: usize = 1024;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RecordingUsage {
    pub retained_bytes: usize,
    pub work: usize,
}

impl RecordingUsage {
    pub(super) fn admitted(self, extra: Self) -> CrawlResult<Self> {
        let retained_bytes = add(self.retained_bytes, extra.retained_bytes)?;
        let work = add(self.work, extra.work)?;
        check(
            "recorded retained bytes",
            retained_bytes,
            MAX_RECORDED_BYTES,
        )?;
        check("recorded effects", work, MAX_RECORDED_WORK)?;
        Ok(Self {
            retained_bytes,
            work,
        })
    }
}

pub(super) fn measure(
    rows: &[RecordedBatch],
    journal: &[RecordedJournal],
) -> CrawlResult<RecordingUsage> {
    let rows = rows
        .iter()
        .try_fold(RecordingUsage::default(), |total, batch| {
            let storage = multiply(batch.rows.capacity(), size_of::<serde_json::Value>())?;
            let payload = batch
                .rows
                .iter()
                .try_fold(storage, |bytes, value| add(bytes, values::retained(value)?))?;
            total.admitted(RecordingUsage {
                retained_bytes: add(payload, EFFECT_STORAGE)?,
                work: batch.rows.len(),
            })
        })?;
    journal.iter().try_fold(rows, |total, entry| {
        let strings = add(entry.phase.capacity(), entry.key.capacity())?;
        let payload = add(strings, values::retained(&entry.payload)?)?;
        total.admitted(RecordingUsage {
            retained_bytes: add(payload, EFFECT_STORAGE)?,
            work: 1,
        })
    })
}

pub(super) fn add(left: usize, right: usize) -> CrawlResult<usize> {
    left.checked_add(right)
        .ok_or_else(|| resource("recording arithmetic", usize::MAX, MAX_RECORDED_BYTES))
}

pub(super) fn multiply(left: usize, right: usize) -> CrawlResult<usize> {
    left.checked_mul(right)
        .ok_or_else(|| resource("recording arithmetic", usize::MAX, MAX_RECORDED_BYTES))
}

pub(super) fn check(
    resource_name: &'static str,
    requested: usize,
    limit: usize,
) -> CrawlResult<()> {
    if requested > limit {
        return Err(resource(resource_name, requested, limit));
    }
    Ok(())
}

pub(super) fn resource(resource: &'static str, requested: usize, limit: usize) -> CrawlError {
    CrawlError::Resource {
        resource,
        requested,
        limit,
    }
}
