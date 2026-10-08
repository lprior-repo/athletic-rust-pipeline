use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use fjall::{Database, Keyspace};

use super::format::{open_format, read_format, StoreFormat, DEFAULT_CACHE_BYTES};
use super::keys::Layout;
use super::rows::{count_rows, walk_keys, TableWalk};
use super::{generation, meta, Store, StoreResult, Table};

pub struct StoreInspection {
    root: PathBuf,
    db: Database,
    entities: Keyspace,
    meta: Keyspace,
    format: StoreFormat,
    evidence_generation: u64,
    derived_generation: u64,
}

impl Store {
    pub fn inspect(root: impl AsRef<Path>) -> StoreResult<StoreInspection> {
        Self::inspect_with(root, DEFAULT_CACHE_BYTES)
    }

    pub fn inspect_with(root: impl AsRef<Path>, cache_bytes: u64) -> StoreResult<StoreInspection> {
        let root = root.as_ref().to_path_buf();
        let (db, entities, journal, meta_keyspace, receipts) =
            super::open_keyspaces(&root, cache_bytes)?;
        open_format(&root, &db, [&entities, &journal, &receipts], &meta_keyspace)?;
        let format = read_format(&meta_keyspace)?;
        let generations = generation::Generations::seeded(&meta_keyspace)?;
        Ok(StoreInspection {
            root,
            db,
            entities,
            meta: meta_keyspace,
            format,
            evidence_generation: generations.entities(),
            derived_generation: generations.derived_current(),
        })
    }
}

impl StoreInspection {
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn format(&self) -> &StoreFormat {
        &self.format
    }

    pub fn evidence_generation(&self) -> u64 {
        self.evidence_generation
    }

    pub fn derived_generation(&self) -> u64 {
        self.derived_generation
    }

    pub fn staging_was_guarded(&self) -> bool {
        self.format.versioned()
    }

    pub fn table_row_counts(&self) -> StoreResult<BTreeMap<String, u64>> {
        let layout = Layout {
            derived_generation: self.derived_generation,
        };
        let mut counts = BTreeMap::new();
        for table in Table::ALL {
            counts.insert(
                table.file().to_string(),
                count_rows(&self.entities, layout, table)?,
            );
        }
        Ok(counts)
    }

    pub fn walk_table(&self, table: Table) -> StoreResult<TableWalk> {
        walk_keys(
            &self.entities,
            Layout {
                derived_generation: self.derived_generation,
            },
            table,
        )
    }

    pub fn next_staging_generation(&self) -> StoreResult<u64> {
        meta::get_u64(&self.meta, generation::DERIVED_NEXT)?.ok_or_else(|| {
            super::StoreError::SchemaUnknown {
                detail: format!(
                    "{} is missing from a versioned store",
                    generation::DERIVED_NEXT
                ),
            }
        })
    }

    pub fn disk_space(&self) -> StoreResult<u64> {
        self.db
            .disk_space()
            .map_err(|source| super::StoreError::Read { source })
    }
}
