use super::membership::{JournalNamespaceContents, JournalNamespaceContentsInventory};
use super::{JournalNamespace, JournalRawEnvelope, ReadBudget, ReadCounts};
use crate::{JournalReadBudget, StoreResult};

pub(super) mod legacy;
mod live;
pub(crate) mod sealed {
    pub(crate) trait Sealed {}
}

pub(crate) enum RetainedJournalBytes<'a> {
    Live(fjall::Slice),
    Frozen(&'a [u8]),
}

impl AsRef<[u8]> for RetainedJournalBytes<'_> {
    fn as_ref(&self) -> &[u8] {
        match self {
            Self::Live(bytes) => bytes.as_ref(),
            Self::Frozen(bytes) => bytes,
        }
    }
}

pub(crate) trait RetainedJournalReadSource: sealed::Sealed {
    fn journal_size(&self, key: &[u8]) -> StoreResult<Option<u32>>;
    fn journal_get(&self, key: &[u8]) -> StoreResult<Option<RetainedJournalBytes<'_>>>;
    fn scan_journal_namespace<G>(
        &self,
        namespace: JournalNamespace<'_>,
        budget: ReadBudget,
        state: &mut G,
        admit: impl FnMut(&mut G) -> StoreResult<()>,
        visit: impl FnMut(&mut G, &[u8], &[u8]) -> StoreResult<()>,
    ) -> StoreResult<JournalNamespaceContents>;
    fn visit_receipt_prefix<G>(
        &self,
        prefix: &[u8],
        budget: ReadBudget,
        state: &mut G,
        admit: impl FnMut(&mut G) -> StoreResult<()>,
        visit: impl FnMut(&mut G, &[u8], &[u8]) -> StoreResult<()>,
    ) -> StoreResult<ReadCounts>;
}

pub(crate) fn read_retained_journal_entry_raw_bounded(
    source: &impl RetainedJournalReadSource,
    phase: &str,
    key: &str,
    budget: JournalReadBudget,
) -> StoreResult<Option<JournalRawEnvelope>> {
    legacy::payload_bounded(source, phase, key, budget, |path, what, raw| {
        super::legacy_raw::envelope::entry(path, what, raw, budget)
    })
}

pub(crate) fn for_each_retained_journal_root_with_admission<G>(
    source: &impl RetainedJournalReadSource,
    namespace: JournalNamespace<'_>,
    budget: ReadBudget,
    state: &mut G,
    admit: impl FnMut(&mut G) -> StoreResult<()>,
    visit: impl FnMut(&mut G, &str) -> StoreResult<()>,
) -> StoreResult<JournalNamespaceContentsInventory> {
    super::membership::retained_roots(source, namespace, budget, state, admit, visit)
}
