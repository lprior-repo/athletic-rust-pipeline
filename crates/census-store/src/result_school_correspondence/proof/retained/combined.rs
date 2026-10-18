use super::*;
use crate::FrozenRetainedCaptureSourceV1;

pub(in super::super) struct Combined<'a, 'b> {
    external: &'a Source<'b>,
    attachments: &'a FrozenRetainedCaptureSourceV1<'b>,
}

impl<'a, 'b> Combined<'a, 'b> {
    pub(in super::super) fn new(
        external: &'a Source<'b>,
        attachments: &'a FrozenRetainedCaptureSourceV1<'b>,
    ) -> Self {
        Self {
            external,
            attachments,
        }
    }
}

impl retained_source_sealed::Sealed for Combined<'_, '_> {}
impl RetainedJournalReadSource for Combined<'_, '_> {
    fn journal_size(&self, key: &[u8]) -> StoreResult<Option<u32>> {
        if self.external.owns_key(key) {
            self.external.journal_size(key)
        } else {
            self.attachments.journal_size(key)
        }
    }

    fn journal_get(&self, key: &[u8]) -> StoreResult<Option<RetainedJournalBytes<'_>>> {
        if self.external.owns_key(key) {
            self.external.journal_get(key)
        } else {
            self.attachments.journal_get(key)
        }
    }

    fn scan_journal_namespace<G>(
        &self,
        namespace: JournalNamespace<'_>,
        budget: ReadBudget,
        state: &mut G,
        admit: impl FnMut(&mut G) -> StoreResult<()>,
        visit: impl FnMut(&mut G, &[u8], &[u8]) -> StoreResult<()>,
    ) -> StoreResult<JournalNamespaceContents> {
        if self.external.owns_namespace(namespace) {
            self.external
                .scan_journal_namespace(namespace, budget, state, admit, visit)
        } else {
            self.attachments
                .scan_journal_namespace(namespace, budget, state, admit, visit)
        }
    }

    fn visit_receipt_prefix<G>(
        &self,
        prefix: &[u8],
        budget: ReadBudget,
        state: &mut G,
        admit: impl FnMut(&mut G) -> StoreResult<()>,
        visit: impl FnMut(&mut G, &[u8], &[u8]) -> StoreResult<()>,
    ) -> StoreResult<ReadCounts> {
        self.attachments
            .visit_receipt_prefix(prefix, budget, state, admit, visit)
    }
}
