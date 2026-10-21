use super::*;
use crate::StoreError;
use fjall::{Keyspace, Readable, Snapshot};

pub(crate) struct PhysicalRetainedJournalSource<'a> {
    snapshot: &'a Snapshot,
    journal: &'a Keyspace,
    receipts: &'a Keyspace,
}

impl<'a> PhysicalRetainedJournalSource<'a> {
    pub(crate) fn new(
        snapshot: &'a Snapshot,
        journal: &'a Keyspace,
        receipts: &'a Keyspace,
    ) -> Self {
        Self {
            snapshot,
            journal,
            receipts,
        }
    }
}

impl sealed::Sealed for PhysicalRetainedJournalSource<'_> {}

impl RetainedJournalReadSource for PhysicalRetainedJournalSource<'_> {
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
        super::super::membership::scan_contents(
            self.snapshot,
            self.journal,
            namespace,
            budget,
            state,
            |_, _| Ok(()),
            admit,
            visit,
        )
    }

    fn visit_receipt_prefix<G>(
        &self,
        prefix: &[u8],
        budget: ReadBudget,
        state: &mut G,
        admit: impl FnMut(&mut G) -> StoreResult<()>,
        visit: impl FnMut(&mut G, &[u8], &[u8]) -> StoreResult<()>,
    ) -> StoreResult<ReadCounts> {
        super::super::receipt_prefix::scan_receipts(
            self.snapshot,
            self.receipts,
            prefix,
            budget,
            state,
            admit,
            visit,
        )
    }
}
