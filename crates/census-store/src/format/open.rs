use super::*;

impl Store {
    pub fn format(root: impl AsRef<Path>) -> StoreResult<StoreFormat> {
        let (db, _entities, _journal, meta_keyspace, _receipts) =
            crate::open_keyspaces(root.as_ref(), DEFAULT_CACHE_BYTES)?;
        let format = read_format(&meta_keyspace)?;
        drop(db);
        Ok(format)
    }

    pub fn open(root: impl AsRef<Path>) -> StoreResult<Self> {
        Self::open_with(root, DEFAULT_CACHE_BYTES)
    }

    pub fn open_with(root: impl AsRef<Path>, cache_bytes: u64) -> StoreResult<Self> {
        let root = root.as_ref().to_path_buf();
        if root.join("fjall/version").is_file() {
            return existing::existing(&root, existing::OpenPurpose::LegacyExecution, cache_bytes);
        }
        let (db, entities, journal, meta_keyspace, receipts) =
            crate::open_keyspaces(&root, cache_bytes)?;
        let format = open_format(&root, &db, [&entities, &journal, &receipts], &meta_keyspace)?;
        let state = crate::installation::validate_format(
            &db.snapshot(),
            &meta_keyspace,
            &journal,
            format.lifecycle_revision,
        )?;
        crate::installation::require_legacy_execution(&state)?;
        crate::ensure_dirs(&root)?;
        Self::recover_opened(root, db, entities, journal, meta_keyspace, receipts)
    }

    pub(super) fn recover_opened(
        root: std::path::PathBuf,
        db: Database,
        entities: Keyspace,
        journal: Keyspace,
        meta_keyspace: Keyspace,
        receipts: Keyspace,
    ) -> StoreResult<Self> {
        crate::read::sweep_stale_temporaries(&root, &meta_keyspace)?;
        let generations = generation::Generations::seeded(&meta_keyspace)?;
        let sequences = crate::sequences::Counters::seeded(&db, &entities, &meta_keyspace)?;
        let store = Self {
            root,
            db,
            entities,
            journal,
            meta: meta_keyspace,
            receipts,
            sequences: std::sync::Arc::new(sequences),
            generations: std::sync::Arc::new(generations),
            appends: std::sync::Arc::new(std::sync::Mutex::new(())),
            staging: std::sync::Arc::new(std::sync::Mutex::new(())),
        };
        store.seed_row_marks()?;
        Ok(store)
    }
}
