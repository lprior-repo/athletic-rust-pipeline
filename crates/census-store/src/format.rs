use fjall::{Database, Keyspace, PersistMode};
use serde::Serialize;
use std::path::Path;

use super::clock::{Clock, SystemClock};
use super::{generation, meta, Store, StoreError, StoreResult};

mod migrate;

pub use migrate::MigrationReport;

pub const STORE_SCHEMA_VERSION: u32 = 1;
pub const KEY_FORMAT_VERSION: u32 = 1;

pub const DEFAULT_CACHE_BYTES: u64 = 1024 * 1024 * 1024;

const STORE_VERSION: &str = "schema:store_version";
const KEY_FORMAT: &str = "schema:key_format";
const CREATED_BY: &str = "schema:created_by";
const CREATED_AT: &str = "schema:created_at";
pub(super) const MIGRATING_TO: &str = "schema:migrating_to";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoreFormat {
    pub schema_version: Option<u32>,
    pub key_format: Option<u32>,
    pub created_by: Option<String>,
    pub created_at: Option<String>,
    pub migration_target: Option<u32>,
}

impl StoreFormat {
    pub fn versioned(&self) -> bool {
        self.schema_version.is_some()
    }
}

enum Verdict {
    Current,
    Newer {
        what: &'static str,
        found: u32,
        supported: u32,
    },
    Migrate(String),
    Unknown(String),
}

pub(super) fn open_format(
    root: &Path,
    db: &Database,
    entities: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    meta_keyspace: &Keyspace,
) -> StoreResult<StoreFormat> {
    let format = read_format(meta_keyspace)?;
    match classify(&format) {
        Verdict::Current => Ok(format),
        Verdict::Newer {
            what,
            found,
            supported,
        } => Err(StoreError::FormatNewer {
            what,
            found,
            supported,
        }),
        Verdict::Migrate(detail) => {
            if store_is_empty(entities, journal, receipts)? {
                initialize(db, meta_keyspace)?;
                return read_format(meta_keyspace);
            }
            Err(StoreError::MigrationRequired {
                root: root.display().to_string(),
                detail,
            })
        }
        Verdict::Unknown(detail) => Err(StoreError::SchemaUnknown { detail }),
    }
}

fn store_is_empty(
    entities: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
) -> StoreResult<bool> {
    let empty = |keyspace: &Keyspace| -> StoreResult<bool> {
        keyspace
            .is_empty()
            .map_err(|source| StoreError::Read { source })
    };
    Ok(empty(entities)? && empty(journal)? && empty(receipts)?)
}

pub(super) fn read_format(meta_keyspace: &Keyspace) -> StoreResult<StoreFormat> {
    Ok(StoreFormat {
        schema_version: meta::get_u32(meta_keyspace, STORE_VERSION)?,
        key_format: meta::get_u32(meta_keyspace, KEY_FORMAT)?,
        created_by: meta::get_text(meta_keyspace, CREATED_BY)?,
        created_at: meta::get_text(meta_keyspace, CREATED_AT)?,
        migration_target: meta::get_u32(meta_keyspace, MIGRATING_TO)?,
    })
}

fn classify(format: &StoreFormat) -> Verdict {
    if let Some(target) = format.migration_target {
        return Verdict::Migrate(format!(
            "an earlier migration to version {target} did not finish; run the store migration to resume it"
        ));
    }
    match (format.schema_version, format.key_format) {
        (None, None) => {
            if format.created_by.is_some() || format.created_at.is_some() {
                Verdict::Unknown("schema version is missing beside creation rows".to_string())
            } else {
                Verdict::Migrate("the store predates versioned schemas".to_string())
            }
        }
        (Some(version), Some(key_format)) => {
            if format.created_by.is_none() || format.created_at.is_none() {
                return Verdict::Unknown("creation lineage rows are missing".to_string());
            }
            if version > STORE_SCHEMA_VERSION {
                return Verdict::Newer {
                    what: "schema version",
                    found: version,
                    supported: STORE_SCHEMA_VERSION,
                };
            }
            if key_format > KEY_FORMAT_VERSION {
                return Verdict::Newer {
                    what: "key format",
                    found: key_format,
                    supported: KEY_FORMAT_VERSION,
                };
            }
            if version < STORE_SCHEMA_VERSION || key_format < KEY_FORMAT_VERSION {
                return Verdict::Migrate(format!(
                    "the store records schema version {version} and key format {key_format}"
                ));
            }
            Verdict::Current
        }
        (Some(version), None) => Verdict::Unknown(format!(
            "schema version {version} is recorded without a key format"
        )),
        (None, Some(key_format)) => Verdict::Unknown(format!(
            "key format {key_format} is recorded without a schema version"
        )),
    }
}

fn initialize(db: &Database, meta_keyspace: &Keyspace) -> StoreResult<()> {
    let mut batch = db.batch();
    meta::put_text(
        &mut batch,
        meta_keyspace,
        STORE_VERSION,
        &STORE_SCHEMA_VERSION.to_string(),
    );
    meta::put_text(
        &mut batch,
        meta_keyspace,
        KEY_FORMAT,
        &KEY_FORMAT_VERSION.to_string(),
    );
    meta::put_text(
        &mut batch,
        meta_keyspace,
        CREATED_BY,
        env!("CARGO_PKG_VERSION"),
    );
    meta::put_text(
        &mut batch,
        meta_keyspace,
        CREATED_AT,
        &SystemClock.today_iso8601(),
    );
    generation::write_seed(&mut batch, meta_keyspace, 0, 0, 1, 1);
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })
}

impl Store {
    pub fn format(root: impl AsRef<Path>) -> StoreResult<StoreFormat> {
        let (db, _entities, _journal, meta_keyspace, _receipts) =
            super::open_keyspaces(root.as_ref(), DEFAULT_CACHE_BYTES)?;
        let format = read_format(&meta_keyspace)?;
        drop(db);
        Ok(format)
    }

    pub fn open(root: impl AsRef<Path>) -> StoreResult<Self> {
        Self::open_with(root, DEFAULT_CACHE_BYTES)
    }

    pub fn open_with(root: impl AsRef<Path>, cache_bytes: u64) -> StoreResult<Self> {
        let root = root.as_ref().to_path_buf();
        super::ensure_dirs(&root)?;
        let (db, entities, journal, meta_keyspace, receipts) =
            super::open_keyspaces(&root, cache_bytes)?;
        open_format(&root, &db, &entities, &journal, &receipts, &meta_keyspace)?;
        super::read::sweep_stale_temporaries(&root)?;
        let generations = generation::Generations::seeded(&meta_keyspace)?;
        let sequences = super::sequences::Counters::seeded(&db, &entities, &meta_keyspace)?;
        let store = Self {
            root,
            db,
            entities,
            journal,
            meta: meta_keyspace,
            receipts,
            sequences,
            generations,
            appends: std::sync::Mutex::new(()),
            staging: std::sync::Mutex::new(()),
        };
        store.seed_row_marks()?;
        Ok(store)
    }
}
