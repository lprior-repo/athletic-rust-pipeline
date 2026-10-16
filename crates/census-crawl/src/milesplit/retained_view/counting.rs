use super::*;
use std::cell::RefCell;

pub(crate) struct CountingJournal<'a, V> {
    view: &'a V,
    reads: RefCell<BTreeMap<(String, String), usize>>,
}

impl<'a, V> CountingJournal<'a, V> {
    pub(crate) fn new(view: &'a V) -> Self {
        Self {
            view,
            reads: RefCell::new(BTreeMap::new()),
        }
    }

    pub(crate) fn reads_for(&self, phase: &str) -> usize {
        self.reads
            .borrow()
            .iter()
            .filter(|((actual, _), _)| actual == phase)
            .map(|(_, count)| count)
            .sum()
    }

    pub(crate) fn reads_of(&self, phase: &str, key: &str) -> usize {
        self.reads
            .borrow()
            .get(&(phase.into(), key.into()))
            .copied()
            .map_or(0, |count| count)
    }
}

impl<V> sealed::Sealed for CountingJournal<'_, V> {}
impl<V: RetainedJournalView> RetainedJournalView for CountingJournal<'_, V> {
    fn store_identity(&self) -> StoreResult<StoreIdentity> {
        self.view.store_identity()
    }
    fn sequence(&self) -> u64 {
        self.view.sequence()
    }
    fn journal_payload_bounded(
        &self,
        phase: &str,
        key: &str,
        budget: JournalReadBudget,
    ) -> StoreResult<Option<Value>> {
        let mut reads = self.reads.borrow_mut();
        let count = reads.entry((phase.into(), key.into())).or_default();
        *count = count.checked_add(1).ok_or(StoreError::CounterOverflow)?;
        self.view.journal_payload_bounded(phase, key, budget)
    }
}
