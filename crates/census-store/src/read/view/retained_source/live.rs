use super::*;
use crate::{StoreError, StoreSnapshot};
use fjall::Readable;

impl sealed::Sealed for StoreSnapshot<'_> {}

impl RetainedJournalReadSource for StoreSnapshot<'_> {
    fn journal_size(&self, key: &[u8]) -> StoreResult<Option<u32>> {
        self.snapshot
            .size_of(self.journal, key)
            .map_err(|source| StoreError::Read { source })
    }

    fn journal_get(&self, key: &[u8]) -> StoreResult<Option<RetainedJournalBytes<'_>>> {
        self.snapshot
            .get(self.journal, key)
            .map(|value| value.map(RetainedJournalBytes::Live))
            .map_err(|source| StoreError::Read { source })
    }

    fn scan_journal_namespace<G>(
        &self,
        namespace: JournalNamespace<'_>,
        budget: ReadBudget,
        state: &mut G,
        admit: impl FnMut(&mut G) -> StoreResult<()>,
        visit: impl FnMut(&mut G, &[u8], &[u8]) -> StoreResult<()>,
    ) -> StoreResult<JournalNamespaceContents> {
        self.scan_journal_namespace_contents(namespace, budget, state, admit, visit)
    }

    fn visit_receipt_prefix<G>(
        &self,
        prefix: &[u8],
        budget: ReadBudget,
        state: &mut G,
        admit: impl FnMut(&mut G) -> StoreResult<()>,
        visit: impl FnMut(&mut G, &[u8], &[u8]) -> StoreResult<()>,
    ) -> StoreResult<ReadCounts> {
        self.visit_attachment_receipts_with_admission(prefix, budget, state, admit, visit)
    }
}
