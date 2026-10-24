use census_domain::model::StoreIdentity;
use census_store::{JournalReadBudget, StoreError, StoreResult, StoreSnapshot};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

mod codec;
mod demand;
mod frozen;
mod recording;
mod sealed {
    pub trait Sealed {}
}

pub use frozen::FrozenRetainedJournal;
pub use recording::RecordingRetainedJournal;

const MAX_BYTES: usize = 512 * 1024 * 1024;
const MAX_ENTRIES: usize = 262_144;
const MAX_PAYLOAD: usize = 4 * 1024 * 1024;

pub trait RetainedJournalView: sealed::Sealed {
    fn store_identity(&self) -> StoreResult<StoreIdentity>;
    fn sequence(&self) -> u64;
    fn journal_payload_bounded(
        &self,
        phase: &str,
        key: &str,
        budget: JournalReadBudget,
    ) -> StoreResult<Option<Value>>;
}

impl sealed::Sealed for StoreSnapshot<'_> {}
impl RetainedJournalView for StoreSnapshot<'_> {
    fn store_identity(&self) -> StoreResult<StoreIdentity> {
        StoreSnapshot::store_identity(self)
    }
    fn sequence(&self) -> u64 {
        StoreSnapshot::sequence(self)
    }
    fn journal_payload_bounded(
        &self,
        phase: &str,
        key: &str,
        budget: JournalReadBudget,
    ) -> StoreResult<Option<Value>> {
        StoreSnapshot::journal_payload_bounded(self, phase, key, budget)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetainedJournalBundle {
    version: u32,
    store_identity: StoreIdentity,
    snapshot_sequence: u64,
    #[serde(deserialize_with = "codec::entries")]
    entries: Vec<Entry>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    phase: String,
    key: String,
    payload: RecordedPayload,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "presence", rename_all = "snake_case", deny_unknown_fields)]
enum RecordedPayload {
    Absent,
    Present {
        value: Value,
        minimum_successful_read_budget: usize,
    },
}

impl RetainedJournalBundle {
    pub fn measure_present(view: &StoreSnapshot<'_>) -> StoreResult<impl Serialize> {
        Self::measure_present_with_progress(view, |_| Ok(()))
    }

    pub fn measure_present_with_progress(
        view: &StoreSnapshot<'_>,
        observer: impl FnMut(&'static str) -> StoreResult<()>,
    ) -> StoreResult<impl Serialize> {
        demand::measure_retained_journal_present(view, observer)
    }

    pub fn encode(&self) -> StoreResult<Vec<u8>> {
        let mut bytes = codec::LimitedBytes::default();
        serde_json::to_writer(&mut bytes, self).map_err(encoding)?;
        Ok(bytes.finish())
    }
}

fn invalid(detail: &str) -> StoreError {
    StoreError::Refused {
        detail: detail.into(),
    }
}
fn encoding(source: serde_json::Error) -> StoreError {
    StoreError::Json {
        detail: "retained source journal bundle".into(),
        source,
    }
}
fn bound(needed: usize, max: usize, what: &'static str) -> StoreResult<()> {
    if needed > max {
        return Err(StoreError::BudgetExceeded {
            what,
            needed: u64::try_from(needed).map_err(|_| StoreError::CounterOverflow)?,
            available: u64::try_from(max).map_err(|_| StoreError::CounterOverflow)?,
        });
    }
    Ok(())
}

fn key(phase: &str, key: &str) -> StoreResult<(String, String)> {
    if !matches!(phase, super::OWNED_CAPTURE_PHASE | super::OWNED_MEET_PHASE)
        || key.is_empty()
        || key.len() > 512
        || key.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err(invalid("unsupported retained source journal key"));
    }
    Ok((phase.into(), key.into()))
}

#[cfg(test)]
mod tests;
