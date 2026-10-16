use super::*;
use std::cell::RefCell;

pub struct RecordingRetainedJournal<'a, 's> {
    view: &'a StoreSnapshot<'s>,
    entries: RefCell<BTreeMap<(String, String), RecordedPayload>>,
    bytes: std::cell::Cell<usize>,
    health: std::cell::Cell<RecordingHealth>,
}

#[derive(Clone, Copy)]
enum RecordingHealth {
    Complete,
    ReadFailed,
}

impl<'a, 's> RecordingRetainedJournal<'a, 's> {
    pub fn new(view: &'a StoreSnapshot<'s>) -> Self {
        Self {
            view,
            entries: RefCell::new(BTreeMap::new()),
            bytes: std::cell::Cell::new(0),
            health: std::cell::Cell::new(RecordingHealth::Complete),
        }
    }

    pub fn finish(self) -> StoreResult<RetainedJournalBundle> {
        if matches!(self.health.get(), RecordingHealth::ReadFailed) {
            return Err(invalid(
                "retained journal recording cannot freeze an unrecorded failed read",
            ));
        }
        Ok(RetainedJournalBundle {
            version: 1,
            store_identity: self.view.store_identity()?,
            snapshot_sequence: self.view.sequence(),
            entries: self
                .entries
                .into_inner()
                .into_iter()
                .map(|((phase, key), payload)| Entry {
                    phase,
                    key,
                    payload,
                })
                .collect(),
        })
    }
}

impl RecordingRetainedJournal<'_, '_> {
    fn read_and_record(
        &self,
        phase: &str,
        name: &str,
        budget: JournalReadBudget,
    ) -> StoreResult<Option<Value>> {
        let key = key(phase, name)?;
        let payload = self.view.journal_payload_bounded(phase, name, budget)?;
        let mut entries = self
            .entries
            .try_borrow_mut()
            .map_err(|_| invalid("nested retained journal recording"))?;
        let recorded = match &payload {
            Some(value) => RecordedPayload::Present {
                value: value.clone(),
                minimum_successful_read_budget: budget.max_encoded_bytes(),
            },
            None => RecordedPayload::Absent,
        };
        if let Some(previous) = entries.get_mut(&key) {
            reconcile(previous, &recorded)?;
            return Ok(payload);
        }
        bound(
            entries
                .len()
                .checked_add(1)
                .ok_or(StoreError::CounterOverflow)?,
            MAX_ENTRIES,
            "retained journal entries",
        )?;
        let size = serde_json::to_vec(&recorded).map_err(encoding)?.len();
        bound(size, MAX_PAYLOAD, "retained journal payload bytes")?;
        let bytes = self
            .bytes
            .get()
            .checked_add(size)
            .ok_or(StoreError::CounterOverflow)?;
        bound(bytes, MAX_BYTES, "retained journal recording bytes")?;
        entries.insert(key, recorded);
        self.bytes.set(bytes);
        Ok(payload)
    }
}

fn reconcile(previous: &mut RecordedPayload, current: &RecordedPayload) -> StoreResult<()> {
    match (previous, current) {
        (RecordedPayload::Absent, RecordedPayload::Absent) => Ok(()),
        (
            RecordedPayload::Present {
                value,
                minimum_successful_read_budget,
            },
            RecordedPayload::Present {
                value: current,
                minimum_successful_read_budget: now,
            },
        ) if value == current => {
            *minimum_successful_read_budget = (*minimum_successful_read_budget).min(*now);
            Ok(())
        }
        _ => Err(invalid("pinned retained journal changed during recording")),
    }
}

impl super::sealed::Sealed for RecordingRetainedJournal<'_, '_> {}
impl RetainedJournalView for RecordingRetainedJournal<'_, '_> {
    fn store_identity(&self) -> StoreResult<StoreIdentity> {
        self.view.store_identity()
    }
    fn sequence(&self) -> u64 {
        self.view.sequence()
    }
    fn journal_payload_bounded(
        &self,
        phase: &str,
        name: &str,
        budget: JournalReadBudget,
    ) -> StoreResult<Option<Value>> {
        let result = self.read_and_record(phase, name, budget);
        if result.is_err() {
            self.health.set(RecordingHealth::ReadFailed);
        }
        result
    }
}
