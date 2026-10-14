use fjall::{Database, Keyspace, PersistMode};
use serde::Serialize;
use std::path::Path;

use super::clock::{Clock, SystemClock};
use super::{generation, meta, Store, StoreError, StoreResult};

pub(crate) mod existing;
mod migrate;

pub use migrate::MigrationReport;

pub const STORE_SCHEMA_VERSION: u32 = 2;
pub const KEY_FORMAT_VERSION: u32 = 1;
pub const LIFECYCLE_REVISION: u32 = 4;
const FRESH_LIFECYCLE_REVISION: u32 = 2;

pub const DEFAULT_CACHE_BYTES: u64 = 1024 * 1024 * 1024;

const STORE_VERSION: &str = "schema:store_version";
const KEY_FORMAT: &str = "schema:key_format";
const CREATED_BY: &str = "schema:created_by";
const CREATED_AT: &str = "schema:created_at";
pub(super) const MIGRATING_TO: &str = "schema:migrating_to";
pub(super) const LIFECYCLE_REVISION_KEY: &str = "schema:lifecycle_revision";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoreFormat {
    pub schema_version: Option<u32>,
    pub key_format: Option<u32>,
    pub lifecycle_revision: Option<u32>,
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
    data: [&Keyspace; 3],
    meta_keyspace: &Keyspace,
) -> StoreResult<StoreFormat> {
    let format = read_format(meta_keyspace)?;
    if fresh_directory(&format, data, meta_keyspace)? {
        initialize(db, meta_keyspace)?;
        return read_format(meta_keyspace);
    }
    let format = require_current(root, format)?;
    let [_, journal, receipts] = data;
    let capture = crate::acquisition::validate_format(
        root,
        &db.snapshot(),
        meta_keyspace,
        journal,
        receipts,
        &format,
    )?;
    crate::acquisition::require_legacy(&capture)?;
    crate::installation::validate_format(
        &db.snapshot(),
        meta_keyspace,
        journal,
        format.lifecycle_revision,
    )?;
    Ok(format)
}

pub(crate) fn require_semantic_format(
    root: &Path,
    db: &Database,
    meta: &Keyspace,
    journal: &Keyspace,
) -> StoreResult<StoreFormat> {
    let format = require_current(root, read_format(meta)?)?;
    let capture =
        crate::acquisition::qualification::declaration(&db.snapshot(), meta, journal, &format)?;
    crate::acquisition::require_legacy(&capture)?;
    crate::installation::validate_format(&db.snapshot(), meta, journal, format.lifecycle_revision)?;
    Ok(format)
}

pub(crate) fn require_current(root: &Path, format: StoreFormat) -> StoreResult<StoreFormat> {
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
        Verdict::Migrate(detail) => Err(StoreError::MigrationRequired {
            root: root.display().to_string(),
            detail,
        }),
        Verdict::Unknown(detail) => Err(StoreError::SchemaUnknown { detail }),
    }
}

fn fresh_directory(
    format: &StoreFormat,
    data: [&Keyspace; 3],
    meta: &Keyspace,
) -> StoreResult<bool> {
    if format.schema_version.is_some() || format.migration_target.is_some() {
        return Ok(false);
    }
    data.into_iter()
        .chain(std::iter::once(meta))
        .try_fold(true, |empty, keyspace| {
            Ok(empty
                && keyspace
                    .is_empty()
                    .map_err(|source| StoreError::Read { source })?)
        })
}

pub(crate) fn read_format(meta_keyspace: &Keyspace) -> StoreResult<StoreFormat> {
    Ok(StoreFormat {
        schema_version: meta::get_u32(meta_keyspace, STORE_VERSION)?,
        key_format: meta::get_u32(meta_keyspace, KEY_FORMAT)?,
        lifecycle_revision: meta::get_u32(meta_keyspace, LIFECYCLE_REVISION_KEY)?,
        created_by: meta::get_text(meta_keyspace, CREATED_BY)?,
        created_at: meta::get_text(meta_keyspace, CREATED_AT)?,
        migration_target: meta::get_u32(meta_keyspace, MIGRATING_TO)?,
    })
}

fn classify(format: &StoreFormat) -> Verdict {
    if let Some(verdict) = unsupported_version(format) {
        return verdict;
    }
    if let Some(target) = format.migration_target {
        if format.schema_version == Some(STORE_SCHEMA_VERSION) {
            return Verdict::Unknown(format!(
                "current schema retains an inconsistent migration target {target}"
            ));
        }
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
        (Some(version), Some(key_format)) => classify_versioned(format, version, key_format),
        (Some(version), None) => Verdict::Unknown(format!(
            "schema version {version} is recorded without a key format"
        )),
        (None, Some(key_format)) => Verdict::Unknown(format!(
            "key format {key_format} is recorded without a schema version"
        )),
    }
}

fn unsupported_version(format: &StoreFormat) -> Option<Verdict> {
    [
        (
            "schema version",
            format.schema_version,
            STORE_SCHEMA_VERSION,
        ),
        ("key format", format.key_format, KEY_FORMAT_VERSION),
        (
            "lifecycle revision",
            format.lifecycle_revision,
            LIFECYCLE_REVISION,
        ),
    ]
    .into_iter()
    .find_map(|(what, found, supported)| match found {
        Some(found) if found > supported => Some(Verdict::Newer {
            what,
            found,
            supported,
        }),
        _ => None,
    })
}

fn classify_versioned(format: &StoreFormat, version: u32, key_format: u32) -> Verdict {
    if format.created_by.is_none() || format.created_at.is_none() {
        return Verdict::Unknown("creation lineage rows are missing".to_string());
    }
    if version < STORE_SCHEMA_VERSION || key_format < KEY_FORMAT_VERSION {
        return Verdict::Migrate(format!(
            "the store records schema version {version} and key format {key_format}"
        ));
    }
    Verdict::Current
}

fn initialize(db: &Database, meta_keyspace: &Keyspace) -> StoreResult<()> {
    let mut batch = db.batch();
    stage_initialization(&mut batch, meta_keyspace, FRESH_LIFECYCLE_REVISION);
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })
}

pub(crate) fn stage_initialization(
    batch: &mut fjall::OwnedWriteBatch,
    meta_keyspace: &Keyspace,
    lifecycle: u32,
) {
    meta::put_text(
        batch,
        meta_keyspace,
        STORE_VERSION,
        &STORE_SCHEMA_VERSION.to_string(),
    );
    meta::put_text(
        batch,
        meta_keyspace,
        KEY_FORMAT,
        &KEY_FORMAT_VERSION.to_string(),
    );
    meta::put_text(
        batch,
        meta_keyspace,
        LIFECYCLE_REVISION_KEY,
        &lifecycle.to_string(),
    );
    meta::put_text(batch, meta_keyspace, CREATED_BY, env!("CARGO_PKG_VERSION"));
    meta::put_text(
        batch,
        meta_keyspace,
        CREATED_AT,
        &SystemClock.today_iso8601(),
    );
    generation::write_seed(batch, meta_keyspace, 0, 0, 1, 1);
}

mod open;
