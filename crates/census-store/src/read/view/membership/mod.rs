use crate::{Store, StoreResult, StoreRevision};

use super::{ReadBudget, StoreSnapshot};

mod namespace;
mod roots;
mod scan;

pub use namespace::JournalNamespace;

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
        let mut roots = roots::RootTracker::new(namespace)?;
        let fingerprint = self.scan_journal_namespace(namespace, budget, |key, raw| {
            roots.accept(key, raw, &mut visit)
        })?;
        let logical_roots = roots.finish()?;
        Ok(JournalNamespaceInventory {
            fingerprint,
            logical_roots,
        })
    }
}

#[cfg(test)]
mod tests;
