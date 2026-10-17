use crate::keys::{self, Layout};
use crate::{Store, StoreError, StoreResult, Table};
use fjall::{Readable, Snapshot};

mod acquisition;
mod attachment_inputs;
mod bounded;
mod bounded_page;
pub use bounded_page::{PhysicalObservationCursor, PhysicalObservationPage};
#[path = "../keyed.rs"]
mod keyed;
mod legacy;
mod legacy_raw;
pub(crate) use legacy_raw::envelope::JournalRawEnvelope;
mod legacy_wire;
mod lineage;
mod membership;
mod merge;
pub(crate) use merge::{accept as accept_merged_row, publish as publish_merged_row};
mod object;
mod object_measure;
mod object_read;
mod object_source;
mod paged_generation;
pub(crate) use object::JournalRawRecords;
pub(crate) use object_read::read_context_object;
mod object_wire;
mod operations;
pub(crate) use operations::enrich_retained_observations;
pub(crate) use operations::RetainedEnrichment;
mod public_identity;
pub use public_identity::AcceptedPublicIdentitySnapshot;
mod receipt_prefix;
mod retained_source;
pub(crate) use membership::{
    admit_namespace_budget, JournalNamespaceContents, JournalNamespaceContentsInventory,
    NamespaceScan,
};
pub(crate) use receipt_prefix::ReceiptScan;
pub(crate) use retained_source::sealed as retained_source_sealed;
pub(crate) use retained_source::{
    for_each_retained_journal_root_with_admission, read_retained_journal_entry_raw_bounded,
    RetainedJournalBytes, RetainedJournalReadSource,
};
mod regions;
mod roster_source;
mod run;

pub use bounded::{decode_physical_observation, PhysicalReadAdmissionV1, ReadBudget, ReadCounts};
pub use membership::{JournalNamespace, JournalNamespaceFingerprint, JournalNamespaceInventory};
pub use object::{JournalDigest, JournalEntry, JournalObject, JournalRecord, JournalRecords};

pub struct StoreSnapshot<'s> {
    snapshot: Snapshot,
    entities: &'s fjall::Keyspace,
    journal: &'s fjall::Keyspace,
    receipts: &'s fjall::Keyspace,
    meta: &'s fjall::Keyspace,
    root: &'s std::path::Path,
    derived_generation: u64,
    evidence_generation: u64,
    revision: crate::StoreRevision<'s>,
}

impl<'s> StoreSnapshot<'s> {
    pub(crate) fn new(
        snapshot: Snapshot,
        store: &'s Store,
        derived_generation: u64,
        evidence_generation: u64,
    ) -> Self {
        Self {
            snapshot,
            entities: &store.entities,
            journal: &store.journal,
            receipts: &store.receipts,
            meta: &store.meta,
            root: &store.root,
            derived_generation,
            evidence_generation,
            revision: crate::StoreRevision::captured(
                store,
                evidence_generation,
                derived_generation,
            ),
        }
    }

    pub fn public_identity_input_revisions_v2(
        &self,
    ) -> StoreResult<crate::PublicIdentityInputRevisionsV2> {
        crate::input_revisions::read(&self.snapshot, self.meta)
    }

    pub fn revision(&self) -> crate::StoreRevision<'s> {
        self.revision
    }

    pub fn sequence(&self) -> u64 {
        self.snapshot.seqno()
    }

    pub fn derived_generation(&self) -> u64 {
        self.derived_generation
    }

    pub fn evidence_generation(&self) -> u64 {
        self.evidence_generation
    }

    fn layout(&self) -> Layout {
        Layout {
            derived_generation: self.derived_generation,
        }
    }

    pub fn verify_layout(&self) -> StoreResult<bool> {
        use super::super::{StorageMode, Table};

        let mut checked = false;
        for table in Table::ALL {
            let derived = matches!(
                table.storage_mode(),
                StorageMode::DerivedGeneration | StorageMode::DerivedMap
            );
            let prefix = if derived {
                keys::derived_table_prefix(table)
            } else {
                keys::observation_prefix(table)
            };
            for guard in self.snapshot.prefix(self.entities, &prefix) {
                let (key, value) = guard
                    .into_inner()
                    .map_err(|source| StoreError::Read { source })?;
                checked = true;
                verify_entity_key(table, derived, &key, value.as_ref())?;
            }
        }

        let journal_entries = self.snapshot.prefix(self.journal, b"").count();
        Ok(checked || journal_entries > 0)
    }
}

fn verify_entity_key(table: Table, derived: bool, key: &[u8], value: &[u8]) -> StoreResult<()> {
    let (id, what) = if derived {
        let (_, id) = keys::view_derived_key(table, key).ok_or_else(|| StoreError::Invariant {
            detail: format!(
                "entity key {} is not a derived key for table {}",
                keys::key_label(key),
                table.file()
            ),
        })?;
        (id, "derived observation id")
    } else {
        let (_, id, _) = keys::view_observation_key(key).ok_or_else(|| StoreError::Invariant {
            detail: format!(
                "entity key {} is not an observation key for table {}",
                keys::key_label(key),
                table.file()
            ),
        })?;
        (id, "observation id")
    };
    let id = std::str::from_utf8(id).map_err(|_| StoreError::Invariant {
        detail: format!(
            "entity key {} carries a non-ascii {what} for table {}",
            keys::key_label(key),
            table.file()
        ),
    })?;
    if id.is_empty() {
        return Err(StoreError::Invariant {
            detail: format!(
                "entity key {} carries an empty {what} for table {}",
                keys::key_label(key),
                table.file()
            ),
        });
    }
    keys::observation_id(value)?;
    Ok(())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod raw_physical_tests;

pub(crate) use object_read::{
    read_context_object_bounded, read_frozen_context_object_bounded, ContextReadLimits,
};
