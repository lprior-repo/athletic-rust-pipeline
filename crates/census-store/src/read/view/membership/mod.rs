use crate::{Store, StoreResult, StoreRevision};

use super::{ReadBudget, StoreSnapshot};

mod contents;
mod namespace;
mod roots;
mod scan;

pub(crate) use contents::{JournalNamespaceContents, JournalNamespaceContentsInventory};
pub use namespace::JournalNamespace;
pub(super) use scan::scan_contents;
pub(crate) use scan::{admit as admit_namespace_budget, NamespaceScan};

pub struct JournalNamespaceFingerprint<'s> {
    revision: StoreRevision<'s>,
    sequence: u64,
    rows: u64,
    encoded_bytes: u64,
    digest: [u8; 32],
}

impl JournalNamespaceFingerprint<'_> {
    pub fn physical_rows(&self) -> u64 {
        self.rows
    }

    pub fn encoded_bytes(&self) -> u64 {
        self.encoded_bytes
    }

    pub fn snapshot_sequence(&self) -> u64 {
        self.sequence
    }

    pub fn ensure_origin(&self, store: &Store) -> StoreResult<()> {
        self.revision
            .ensure_origin(store, "compare journal namespace membership")
    }

    pub fn same_contents(
        &self,
        store: &Store,
        other: &JournalNamespaceFingerprint<'_>,
    ) -> StoreResult<bool> {
        self.ensure_origin(store)?;
        other.ensure_origin(store)?;
        Ok(self.rows == other.rows
            && self.encoded_bytes == other.encoded_bytes
            && self.digest == other.digest)
    }
}

pub struct JournalNamespaceInventory<'s> {
    fingerprint: JournalNamespaceFingerprint<'s>,
    logical_roots: u64,
}

impl<'s> JournalNamespaceInventory<'s> {
    pub fn fingerprint(&self) -> &JournalNamespaceFingerprint<'s> {
        &self.fingerprint
    }

    pub fn logical_roots(&self) -> u64 {
        self.logical_roots
    }
}

impl<'s> StoreSnapshot<'s> {
    pub fn journal_namespace_fingerprint(
        &self,
        namespace: JournalNamespace<'_>,
        budget: ReadBudget,
    ) -> StoreResult<JournalNamespaceFingerprint<'s>> {
        self.scan_journal_namespace(namespace, budget, |_, _| Ok(()))
    }

    pub fn for_each_journal_root_bounded(
        &self,
        namespace: JournalNamespace<'_>,
        budget: ReadBudget,
        mut visit: impl FnMut(&str) -> StoreResult<()>,
    ) -> StoreResult<JournalNamespaceInventory<'s>> {
        self.for_each_journal_root_with_admission(
            namespace,
            budget,
            &mut (),
            |_| Ok(()),
            |_, key| visit(key),
        )
    }

    pub(crate) fn for_each_journal_root_with_admission<G>(
        &self,
        namespace: JournalNamespace<'_>,
        budget: ReadBudget,
        state: &mut G,
        admit_row: impl FnMut(&mut G) -> StoreResult<()>,
        visit: impl FnMut(&mut G, &str) -> StoreResult<()>,
    ) -> StoreResult<JournalNamespaceInventory<'s>> {
        let inventory = retained_roots(self, namespace, budget, state, admit_row, visit)?;
        Ok(JournalNamespaceInventory {
            fingerprint: JournalNamespaceFingerprint::from_contents(self, inventory.contents),
            logical_roots: inventory.logical_roots,
        })
    }
}

pub(crate) fn retained_roots<G>(
    source: &impl super::retained_source::RetainedJournalReadSource,
    namespace: JournalNamespace<'_>,
    budget: ReadBudget,
    state: &mut G,
    admit: impl FnMut(&mut G) -> StoreResult<()>,
    mut visit: impl FnMut(&mut G, &str) -> StoreResult<()>,
) -> StoreResult<JournalNamespaceContentsInventory> {
    let mut roots = roots::RootTracker::new(namespace)?;
    let contents =
        source.scan_journal_namespace(namespace, budget, state, admit, |state, key, raw| {
            roots.accept(key, raw, &mut |key| visit(state, key))
        })?;
    let logical_roots = roots.finish()?;
    Ok(JournalNamespaceContentsInventory {
        contents,
        logical_roots,
    })
}

impl<'s> JournalNamespaceFingerprint<'s> {
    pub(super) fn from_contents(
        view: &StoreSnapshot<'s>,
        contents: JournalNamespaceContents,
    ) -> Self {
        Self {
            revision: view.revision(),
            sequence: view.sequence(),
            rows: contents.rows,
            encoded_bytes: contents.encoded_bytes,
            digest: contents.digest,
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod preadmission_tests;
