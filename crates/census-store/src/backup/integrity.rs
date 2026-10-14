use super::{
    profile_integrity, CorruptionClass, IntegrityCorruption, IntegrityReport, IntegrityTable,
};
use crate::journal::{
    CHUNK_TYPE, MANIFEST_TYPE, OBJECT_ALIAS_TYPE, OBJECT_CHUNK_TYPE, OBJECT_INDEX_TYPE,
    OBJECT_MANIFEST_TYPE,
};
use crate::{Store, StoreError, StoreResult, Table};

mod decode;
mod decode_chunks;
mod decode_digest;
mod decode_entry;
mod decode_passes;
mod helpers;
mod journal_files;
mod object;
mod tables;

use self::decode::{Decode, MAX_CHUNK_BYTES};
use self::helpers::{count_at, length};
impl Store {
    pub fn integrity(&self) -> StoreResult<IntegrityReport> {
        self.integrity_with_window(decode::MAX_DECODE_RECORDS, decode::MAX_DECODE_BYTES)
    }

    pub fn integrity_complete(&self) -> StoreResult<IntegrityReport> {
        self.integrity_with_window(u64::MAX, u64::MAX)
    }

    pub(crate) fn integrity_with_window(
        &self,
        record_window: u64,
        byte_window: u64,
    ) -> StoreResult<IntegrityReport> {
        let mut tables = Vec::with_capacity(Table::ALL.len());
        let mut unreadable_journals = Vec::new();
        let unreadable_entity_logs = Vec::new();
        tables::check(self, &mut tables)?;
        journal_files::check(self, &mut unreadable_journals)?;
        let mut decode = decode::Decode::with_window(self, record_window, byte_window);
        decode_passes::journal_pass(&mut decode)?;
        decode_passes::payload_pass(&mut decode)?;
        let decode::Decode {
            mut corruptions,
            mut limits,
            ..
        } = decode;
        let profiles =
            profile_integrity::check(&self.db.snapshot(), &self.meta, record_window, byte_window)?;
        corruptions.extend(profiles.corruptions);
        limits.extend(profiles.limits);
        let tables_sound = tables
            .iter()
            .all(|table| table.expected == table.actual && table.details.is_empty());
        Ok(IntegrityReport {
            ok: tables_sound
                && unreadable_journals.is_empty()
                && unreadable_entity_logs.is_empty()
                && corruptions.is_empty()
                && limits.is_empty(),
            tables,
            unreadable_journals,
            unreadable_entity_logs,
            corruptions,
            decode_limits: limits,
        })
    }
}

#[cfg(test)]
#[path = "integrity_tests.rs"]
mod integrity_tests;
