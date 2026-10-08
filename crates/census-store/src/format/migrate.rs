use std::path::Path;

use serde::Serialize;

use super::{classify, fresh_directory, initialize, read_format, StoreFormat, Verdict};
use super::{KEY_FORMAT_VERSION, MIGRATING_TO, STORE_SCHEMA_VERSION, STORE_VERSION};
use crate::{meta, Store, StoreError, StoreResult};

mod exact;
mod owned;
mod rewrite;
mod time;

#[cfg(test)]
mod exact_tests;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Serialize)]
pub struct MigrationReport {
    pub root: String,
    pub from_schema: Option<u32>,
    pub to_schema: u32,
    pub to_key_format: u32,
    pub rewritten_rows: u64,
    pub dropped_rows: u64,
    pub already_current: bool,
    pub integrity_ok: bool,
}

#[derive(Debug, Default, Clone, Copy)]
struct Rewritten {
    rewritten: u64,
    dropped: u64,
}

impl Rewritten {
    fn combine(self, other: Self) -> StoreResult<Self> {
        Ok(Self {
            rewritten: self
                .rewritten
                .checked_add(other.rewritten)
                .ok_or(StoreError::CounterOverflow)?,
            dropped: self
                .dropped
                .checked_add(other.dropped)
                .ok_or(StoreError::CounterOverflow)?,
        })
    }
}

impl Store {
    pub fn migrate(root: impl AsRef<Path>) -> StoreResult<MigrationReport> {
        Self::migrate_with(root, super::DEFAULT_CACHE_BYTES)
    }

    pub fn migrate_with(root: impl AsRef<Path>, cache_bytes: u64) -> StoreResult<MigrationReport> {
        let root = root.as_ref();
        crate::ensure_dirs(root)?;
        let (format, already_current, rewritten) = migrate_stopped(root, cache_bytes)?;
        let store = Self::open_with(root, cache_bytes)?;
        let integrity = store.integrity()?;
        Ok(MigrationReport {
            root: root.display().to_string(),
            from_schema: format.schema_version,
            to_schema: STORE_SCHEMA_VERSION,
            to_key_format: KEY_FORMAT_VERSION,
            rewritten_rows: rewritten.rewritten,
            dropped_rows: rewritten.dropped,
            already_current,
            integrity_ok: integrity.ok,
        })
    }
}

fn migrate_stopped(root: &Path, cache_bytes: u64) -> StoreResult<(StoreFormat, bool, Rewritten)> {
    let (db, entities, journal, meta, receipts) = crate::open_keyspaces(root, cache_bytes)?;
    let format = read_format(&meta)?;
    let context = time::Context {
        db: &db,
        entities: &entities,
        journal: &journal,
        meta: &meta,
    };
    let already_current = matches!(classify(&format), Verdict::Current);
    let rewritten = match classify(&format) {
        Verdict::Current => Rewritten::default(),
        Verdict::Newer {
            what,
            found,
            supported,
        } => {
            return Err(StoreError::FormatNewer {
                what,
                found,
                supported,
            })
        }
        Verdict::Unknown(detail) => return Err(StoreError::SchemaUnknown { detail }),
        Verdict::Migrate(_) => {
            if fresh_directory(&format, [&entities, &journal, &receipts], &meta)? {
                initialize(&db, &meta)?;
                Rewritten::default()
            } else {
                upgrade(&context, &format)?
            }
        }
    };
    Ok((format, already_current, rewritten))
}

fn upgrade(context: &time::Context<'_>, format: &StoreFormat) -> StoreResult<Rewritten> {
    validate_path(format)?;
    let mut marker = context.db.batch();
    meta::put_text(&mut marker, context.meta, MIGRATING_TO, "2");
    time::commit(marker)?;
    if format.schema_version.is_some_and(|version| version >= 1) {
        crate::generation::Generations::seeded(context.meta)?;
    }
    let legacy = if format.schema_version.map_or(true, |version| version == 0) {
        rewrite::rewrite(context)?
    } else {
        rewrite::statistics(context.meta)?
    };
    let rewritten = legacy.combine(time::migrate(context)?)?;
    let mut batch = context.db.batch();
    meta::put_text(
        &mut batch,
        context.meta,
        STORE_VERSION,
        &STORE_SCHEMA_VERSION.to_string(),
    );
    batch.remove(context.meta, MIGRATING_TO);
    time::clear_cursors(&mut batch, context.meta);
    rewrite::clear_statistics(&mut batch, context.meta);
    time::commit(batch)?;
    Ok(rewritten)
}

fn validate_path(format: &StoreFormat) -> StoreResult<()> {
    if format
        .migration_target
        .is_some_and(|target| target != 1 && target != 2)
        || format
            .schema_version
            .is_some_and(|version| version > STORE_SCHEMA_VERSION)
        || format
            .key_format
            .is_some_and(|version| version > KEY_FORMAT_VERSION)
    {
        return Err(StoreError::SchemaUnknown {
            detail: "unsupported schema migration path".into(),
        });
    }
    if format.schema_version.is_some_and(|version| version >= 1)
        && format.key_format != Some(KEY_FORMAT_VERSION)
    {
        return Err(StoreError::SchemaUnknown {
            detail: "exact-time migration requires key format1".into(),
        });
    }
    match (format.schema_version, format.key_format) {
        (None, None) if format.created_by.is_none() && format.created_at.is_none() => Ok(()),
        (Some(_), Some(_)) if format.created_by.is_some() && format.created_at.is_some() => Ok(()),
        _ => Err(StoreError::SchemaUnknown {
            detail: "migration metadata lacks schema or creation lineage".into(),
        }),
    }
}
