//! Store handle opening: root preparation, keyspace handles and the key-layout
//! gate (ADR-001).
//!
//! The operations the handle hosts live in `operations.rs`; this module owns the
//! state an operation needs and the validation that must pass before any of them
//! runs.
use super::{
    error::{map_database_error, Result, StoreError},
    rankings::common::{RANKING_KEY_REVISION, RANKING_KEY_REVISION_META_KEY},
    root::prepare_root,
    CACHE_BYTES,
};
use fjall::{Database, Keyspace, KeyspaceCreateOptions, PersistMode};
use std::{path::Path, sync::Mutex};

mod operations;

const DOCUMENTS: &str = "documents";
const SOURCES: &str = "source_index";
const ATTEMPTS: &str = "attempts";
const META: &str = "meta";

pub(super) struct StoreInner {
    pub(super) database: Database,
    pub(super) documents: Keyspace,
    pub(super) sources: Keyspace,
    pub(super) attempts: Keyspace,
    pub(super) rankings: Keyspace,
    pub(super) meta: Keyspace,
    pub(super) writer: Mutex<()>,
}

impl StoreInner {
    pub fn open(path: &Path) -> Result<Self> {
        prepare_root(path)?;
        let database = Database::builder(path)
            .cache_size(CACHE_BYTES)
            .manual_journal_persist(true)
            .open()
            .map_err(map_database_error)?;
        let documents = database
            .keyspace(DOCUMENTS, KeyspaceCreateOptions::default)
            .map_err(map_database_error)?;
        let sources = database
            .keyspace(SOURCES, KeyspaceCreateOptions::default)
            .map_err(map_database_error)?;
        let attempts = database
            .keyspace(ATTEMPTS, KeyspaceCreateOptions::default)
            .map_err(map_database_error)?;
        let rankings = database
            .keyspace("rankings", KeyspaceCreateOptions::default)
            .map_err(map_database_error)?;
        let meta = database
            .keyspace(META, KeyspaceCreateOptions::default)
            .map_err(map_database_error)?;
        let store = Self {
            database,
            documents,
            sources,
            attempts,
            rankings,
            meta,
            writer: Mutex::new(()),
        };
        store.enforce_ranking_key_revision()?;
        Ok(store)
    }

    /// Refuse to open a store whose ranking keys were written under another key
    /// encoding (ADR-001: Fjall enforces no constraint for us, so the revision
    /// in `meta` is the only record of the key layout).
    ///
    /// Revision 1 keys carry no checkpoint, and the checkpoint a revision 1 key
    /// belongs to is not recoverable from it, so there is nothing to migrate
    /// from: the layout change is refused rather than misread, and the operator
    /// decides whether to rebuild the rankings keyspace.
    fn enforce_ranking_key_revision(&self) -> Result<()> {
        let marker = self
            .meta
            .get(RANKING_KEY_REVISION_META_KEY)
            .map_err(map_database_error)?;
        match marker {
            Some(stored) => (stored.as_ref() == RANKING_KEY_REVISION.as_bytes())
                .then_some(())
                .ok_or(StoreError::CorruptData),
            None => {
                if !self.rankings.is_empty().map_err(map_database_error)? {
                    return Err(StoreError::CorruptData);
                }
                let mut batch = self.database.batch();
                batch.insert(
                    &self.meta,
                    RANKING_KEY_REVISION_META_KEY,
                    RANKING_KEY_REVISION.as_bytes(),
                );
                batch
                    .durability(Some(PersistMode::SyncAll))
                    .commit()
                    .map_err(map_database_error)
            }
        }
    }
}
