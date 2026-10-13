use std::fs;

use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use serde_json::Value;

use super::{CorruptionClass, IntegrityCorruption, IntegrityReport, IntegrityTable};
use crate::journal::{
    chunk_key, parse_chunk_key, CHUNK_TYPE, MANIFEST_TYPE, OBJECT_ALIAS_TYPE, OBJECT_CHUNK_TYPE,
    OBJECT_INDEX_TYPE, OBJECT_MANIFEST_TYPE,
};
use crate::keys;
use crate::{
    StorageMode, Store, StoreError, StoreResult, Table, TableWalk, MAX_JOURNAL_VALUE_BYTES,
};

mod object;

const MAX_DECODE_RECORDS: u64 = 25_000;
const MAX_DECODE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_CHUNK_BYTES: usize = 512 * 1024;
const MAX_CORRUPTIONS: usize = 1024;
const MAX_LIMITS: usize = 16;
const OWNED_DIGEST_PHASES: [&str; 4] = [
    "milesplit_owned_meet_v1",
    "milesplit_owned_meet_v2",
    "milesplit_owned_meet_v3",
    "milesplit_owned_meet_v4",
];

impl Store {
    pub fn integrity(&self) -> StoreResult<IntegrityReport> {
        self.integrity_with_window(MAX_DECODE_RECORDS, MAX_DECODE_BYTES)
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
        let mut unreadable_entity_logs = Vec::new();
        self.check_table_integrity(&mut tables, &mut unreadable_entity_logs)?;
        self.check_journal_integrity(&mut unreadable_journals)?;
        let mut decode = Decode::with_window(self, record_window, byte_window);
        decode.journal_pass()?;
        decode.payload_pass()?;
        let Decode {
            mut corruptions,
            mut limits,
            ..
        } = decode;
        let registries =
            super::registry::check(&self.db.snapshot(), &self.meta, record_window, byte_window)?;
        corruptions.extend(registries.corruptions);
        limits.extend(registries.limits);
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

    fn check_table_integrity(
        &self,
        tables: &mut Vec<IntegrityTable>,
        _unreadable: &mut Vec<String>,
    ) -> StoreResult<()> {
        for table in Table::ALL {
            let walk = self.walk_table(table)?;
            tables.push(IntegrityTable {
                table: table.file().to_string(),
                expected: self.count(table)?,
                actual: walk.rows,
                details: mode_details(self, table, walk)?,
            });
        }
        Ok(())
    }

    fn check_journal_integrity(&self, unreadable: &mut Vec<String>) -> StoreResult<()> {
        let journal_dir = self.root().join("journal");
        if !journal_dir.exists() {
            return Ok(());
        }
        let entries = fs::read_dir(&journal_dir).map_err(|source| StoreError::Io {
            path: journal_dir.clone(),
            source,
        })?;
        for entry in entries {
            let path = entry
                .map_err(|source| StoreError::Io {
                    path: journal_dir.clone(),
                    source,
                })?
                .path();
            if !path.is_file() {
                continue;
            }
            if fs::File::open(&path).is_err() {
                let name = match path.file_name().map(|n| n.to_string_lossy().to_string()) {
                    Some(value) => value,
                    None => path.display().to_string(),
                };
                unreadable.push(name);
            }
        }
        Ok(())
    }
}

fn mode_details(store: &Store, table: Table, walk: TableWalk) -> StoreResult<Vec<String>> {
    let mut details = Vec::new();
    match table.storage_mode() {
        StorageMode::ObservationLog => {
            let keys = match walk.highest_sequence {
                Some(highest) => highest.checked_add(1).ok_or(StoreError::CounterOverflow)?,
                None => 0,
            };
            let mark = store.sequences.next_sequence(table)?;
            if mark < keys {
                details.push(format!(
                    "sequence mark {mark} is behind the highest key stored ({keys})"
                ));
            }
        }
        StorageMode::DerivedGeneration => {
            if walk.repeated_ids > 0 {
                details.push(format!(
                    "{} ids own more than one row of a generation table",
                    walk.repeated_ids
                ));
            }
        }
        StorageMode::DerivedMap => {
            if walk.foreign_sequences > 0 {
                details.push(format!(
                    "{} rows are keyed under a sequence other than zero",
                    walk.foreign_sequences
                ));
            }
            if walk.repeated_ids > 0 {
                details.push(format!("{} ids own more than one row", walk.repeated_ids));
            }
        }
    }
    Ok(details)
}

struct Decode<'a> {
    store: &'a Store,
    corruptions: Vec<IntegrityCorruption>,
    limits: Vec<String>,
    pass: &'static str,
    record_window: u64,
    byte_window: u64,
    records: u64,
    bytes: u64,
    capped: bool,
    window_closed: bool,
}

impl<'a> Decode<'a> {
    fn with_window(store: &'a Store, record_window: u64, byte_window: u64) -> Self {
        Self {
            store,
            corruptions: Vec::new(),
            limits: Vec::new(),
            pass: "integrity decode",
            record_window,
            byte_window,
            records: 0,
            bytes: 0,
            capped: false,
            window_closed: false,
        }
    }

    fn journal_pass(&mut self) -> StoreResult<()> {
        self.open_window("journal decode");
        let store = self.store;
        for guard in store.journal.iter() {
            let (key, raw) = guard
                .into_inner()
                .map_err(|source| StoreError::Read { source })?;
            if !self.charge(1, length(raw.len())) {
                return Ok(());
            }
            let bytes: &[u8] = key.as_ref();
            let Some(separator) = bytes.iter().position(|byte| *byte == 0) else {
                if !self.note(
                    CorruptionClass::Undecodable,
                    format!("journal/{}", keys::key_label(bytes)),
                    "the journal key carries no phase separator".to_string(),
                ) {
                    return Ok(());
                }
                continue;
            };
            let phase = String::from_utf8_lossy(
                bytes
                    .get(..separator)
                    .map_or(&[][..], core::convert::identity),
            );
            let Some(locator) = bytes.get(separator.saturating_add(1)..) else {
                continue;
            };
            if let Some((logical, index)) = parse_chunk_key(locator) {
                let logical_key = Store::journal_key(&phase, &String::from_utf8_lossy(logical));
                if !store
                    .journal
                    .contains_key(logical_key)
                    .map_err(|source| StoreError::Read { source })?
                {
                    let mut subject = format!("journal/chunk/{}", keys::key_label(bytes));
                    self.read_single_chunk(raw.as_ref(), index, &mut subject);
                }
                continue;
            }
            let locator = String::from_utf8_lossy(locator);
            self.entry(&phase, &locator, &key, raw.as_ref())?;
        }
        Ok(())
    }

    fn payload_pass(&mut self) -> StoreResult<()> {
        self.open_window("payload decode");
        let store = self.store;
        for table in Table::ALL {
            let prefix = store.layout().prefix(table);
            for guard in store.entities.prefix(prefix.as_slice()) {
                let (key, value) = guard
                    .into_inner()
                    .map_err(|source| StoreError::Read { source })?;
                if !self.charge(1, length(value.len())) {
                    return Ok(());
                }
                if let Err(error) = keys::observation_id(value.as_ref()) {
                    if !self.note(
                        CorruptionClass::Undecodable,
                        format!("row/{}/{}", table.file(), keys::key_label(&key)),
                        error.to_string(),
                    ) {
                        return Ok(());
                    }
                }
            }
        }
        Ok(())
    }

    fn entry(&mut self, phase: &str, locator: &str, row_key: &[u8], raw: &[u8]) -> StoreResult<()> {
        let subject = format!("journal/{phase}/{locator}");
        let parsed: Value = match serde_json::from_slice(raw) {
            Ok(parsed) => parsed,
            Err(source) => {
                self.note(CorruptionClass::Undecodable, subject, source.to_string());
                return Ok(());
            }
        };
        match parsed.get("type").and_then(Value::as_str) {
            Some(MANIFEST_TYPE) => self.chunked(phase, locator, row_key, &parsed, subject),
            Some(OBJECT_CHUNK_TYPE) => {
                self.object_chunk(subject, &parsed);
                Ok(())
            }
            Some(OBJECT_MANIFEST_TYPE) => {
                self.object_manifest(subject, &parsed);
                Ok(())
            }
            Some(OBJECT_INDEX_TYPE) => {
                self.object_metadata(subject, &parsed, &["entries"]);
                Ok(())
            }
            Some(OBJECT_ALIAS_TYPE) => self.object_alias(phase, locator, &parsed, subject),
            _ => {
                match parsed.get("payload") {
                    Some(payload) => self.digest(phase, locator, payload, subject),
                    None => {
                        self.note(
                            CorruptionClass::Undecodable,
                            subject,
                            "the journal entry carries no payload".to_string(),
                        );
                    }
                }
                Ok(())
            }
        }
    }

    fn chunked(
        &mut self,
        phase: &str,
        locator: &str,
        row_key: &[u8],
        manifest: &Value,
        mut subject: String,
    ) -> StoreResult<()> {
        let (Some(chunks), Some(total_bytes)) = (
            count_at(manifest, "chunks"),
            count_at(manifest, "total_bytes"),
        ) else {
            self.note(
                CorruptionClass::Undecodable,
                subject,
                "the manifest records no chunk count or payload length".to_string(),
            );
            return Ok(());
        };
        let validation = self.validate_chunked_manifest(chunks, total_bytes, &mut subject);
        if !validation {
            return Ok(());
        }
        let Some(collected) = self.read_chunks(row_key, chunks, total_bytes, &mut subject)? else {
            return Ok(());
        };
        if self.validate_collected_size(collected.len(), total_bytes, &mut subject) {
            self.digest_chunked_payload(phase, locator, collected, subject);
        }
        Ok(())
    }

    fn validate_chunked_manifest(
        &mut self,
        chunks: usize,
        total_bytes: usize,
        subject: &mut String,
    ) -> bool {
        if chunks == 0 {
            self.note(
                CorruptionClass::Truncated,
                std::mem::take(subject),
                "the manifest records no chunks".to_string(),
            );
            return false;
        }
        let ceiling = chunks.saturating_mul(MAX_CHUNK_BYTES);
        if total_bytes > ceiling {
            self.note(
                CorruptionClass::Oversized,
                std::mem::take(subject),
                format!(
                    "the manifest records a {total_bytes}-byte payload for {chunks} chunks, past \
                     the {ceiling}-byte ceiling those chunks carry"
                ),
            );
            return false;
        }
        if !self.charge(0, length(total_bytes)) {
            return false;
        }
        true
    }

    fn read_chunks(
        &mut self,
        row_key: &[u8],
        chunks: usize,
        total_bytes: usize,
        subject: &mut String,
    ) -> StoreResult<Option<Vec<u8>>> {
        let mut collected = Vec::with_capacity(total_bytes);
        for index in 0..chunks {
            let chunk = match chunk_key(row_key, index) {
                Ok(chunk) => chunk,
                Err(source) => {
                    self.note(
                        CorruptionClass::Truncated,
                        std::mem::take(subject),
                        source.to_string(),
                    );
                    return Ok(None);
                }
            };
            if !self.charge(1, 0) {
                return Ok(None);
            }
            let Some(raw) = self
                .store
                .journal
                .get(&chunk)
                .map_err(|source| StoreError::Read { source })?
            else {
                self.note(
                    CorruptionClass::Truncated,
                    std::mem::take(subject),
                    format!("chunk {index} of {chunks} is missing"),
                );
                return Ok(None);
            };
            if !self.charge(0, length(raw.len())) {
                return Ok(None);
            }
            let Some(decoded) = self.read_single_chunk(raw.as_ref(), index, subject) else {
                return Ok(None);
            };
            collected.extend_from_slice(&decoded);
            if collected.len() > total_bytes {
                self.note(
                    CorruptionClass::Oversized,
                    std::mem::take(subject),
                    format!(
                        "the chunks carry more than the {total_bytes} bytes the manifest records"
                    ),
                );
                return Ok(None);
            }
        }
        Ok(Some(collected))
    }

    fn read_single_chunk(
        &mut self,
        raw: &[u8],
        index: usize,
        subject: &mut String,
    ) -> Option<Vec<u8>> {
        if raw.len() > MAX_JOURNAL_VALUE_BYTES {
            self.note(
                CorruptionClass::Oversized,
                std::mem::take(subject),
                format!(
                    "chunk {index} is {} bytes, past the {MAX_JOURNAL_VALUE_BYTES}-byte physical \
                     value ceiling",
                    raw.len()
                ),
            );
            return None;
        }
        let envelope: Value = match serde_json::from_slice(raw) {
            Ok(envelope) => envelope,
            Err(source) => {
                self.note(
                    CorruptionClass::Undecodable,
                    std::mem::take(subject),
                    format!("chunk {index} envelope: {source}"),
                );
                return None;
            }
        };
        if envelope.get("type").and_then(Value::as_str) != Some(CHUNK_TYPE) {
            self.note(
                CorruptionClass::Undecodable,
                std::mem::take(subject),
                format!("chunk {index} is not a chunk envelope"),
            );
            return None;
        }
        let Some(data) = envelope.get("data").and_then(Value::as_str) else {
            self.note(
                CorruptionClass::Undecodable,
                std::mem::take(subject),
                format!("chunk {index} carries no data string"),
            );
            return None;
        };
        self.chunk_bytes(subject, index, data, MAX_CHUNK_BYTES)
    }

    fn validate_collected_size(
        &mut self,
        collected: usize,
        total_bytes: usize,
        subject: &mut String,
    ) -> bool {
        if collected != total_bytes {
            self.note(
                CorruptionClass::Truncated,
                std::mem::take(subject),
                format!(
                    "the reassembled payload is {} bytes against the {total_bytes} the manifest \
                     records",
                    collected
                ),
            );
            return false;
        }
        true
    }

    fn digest_chunked_payload(
        &mut self,
        phase: &str,
        locator: &str,
        collected: Vec<u8>,
        subject: String,
    ) {
        let entry: Value = match serde_json::from_slice(&collected) {
            Ok(entry) => entry,
            Err(source) => {
                self.note(CorruptionClass::Undecodable, subject, source.to_string());
                return;
            }
        };
        match entry.get("payload") {
            Some(payload) => self.digest(phase, locator, payload, subject),
            None => {
                self.note(
                    CorruptionClass::Undecodable,
                    subject,
                    "the journal entry carries no payload".to_string(),
                );
            }
        }
    }

    fn chunk_bytes(
        &mut self,
        subject: &str,
        index: usize,
        data: &str,
        budget: usize,
    ) -> Option<Vec<u8>> {
        let encoded_ceiling = budget.div_ceil(3).saturating_mul(4);
        if data.len() > encoded_ceiling {
            self.note(
                CorruptionClass::Oversized,
                subject.to_string(),
                format!(
                    "chunk {index} carries {} encoded bytes, past the {budget}-byte chunk decode \
                     budget",
                    data.len()
                ),
            );
            return None;
        }
        match BASE64_STANDARD.decode(data) {
            Ok(bytes) if bytes.len() > budget => {
                self.note(
                    CorruptionClass::Oversized,
                    subject.to_string(),
                    format!(
                        "chunk {index} decodes to {} bytes, past the {budget}-byte chunk decode \
                         budget",
                        bytes.len()
                    ),
                );
                None
            }
            Ok(bytes) => Some(bytes),
            Err(source) => {
                self.note(
                    CorruptionClass::Undecodable,
                    subject.to_string(),
                    format!("chunk {index} is not valid base64: {source}"),
                );
                None
            }
        }
    }

    fn digest(&mut self, phase: &str, locator: &str, payload: &Value, subject: String) {
        if !OWNED_DIGEST_PHASES.contains(&phase) || !locator.starts_with("row/") {
            return;
        }
        let Some((_, recorded)) = locator.rsplit_once('/') else {
            return;
        };
        if recorded.len() != 64 || !recorded.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return;
        }
        let actual = match census_domain::model::serialized_digest(payload) {
            Ok(actual) => actual,
            Err(error) => {
                self.note(
                    CorruptionClass::Undecodable,
                    subject,
                    format!(
                        "the payload cannot be hashed for the {recorded} digest its key records: \
                         {error}"
                    ),
                );
                return;
            }
        };
        if actual != recorded {
            self.note(
                CorruptionClass::DigestMismatch,
                subject,
                format!("the key records {recorded} and the payload hashes to {actual}"),
            );
        }
    }

    fn open_window(&mut self, pass: &'static str) {
        self.pass = pass;
        self.records = 0;
        self.bytes = 0;
        self.window_closed = false;
    }

    fn charge(&mut self, records: u64, bytes: u64) -> bool {
        if self.capped || self.window_closed {
            return false;
        }
        let (Some(next_records), Some(next_bytes)) = (
            self.records.checked_add(records),
            self.bytes.checked_add(bytes),
        ) else {
            self.close_window();
            return false;
        };
        if next_records > self.record_window || next_bytes > self.byte_window {
            self.close_window();
            return false;
        }
        self.records = next_records;
        self.bytes = next_bytes;
        true
    }

    fn close_window(&mut self) {
        self.window_closed = true;
        if self.limits.len() < MAX_LIMITS {
            self.limits.push(format!(
                "{} stopped at the ADR-036 integrity profile window ({} records or {} bytes): \
                 content past it is not decoded",
                self.pass, self.record_window, self.byte_window
            ));
        }
    }

    fn note(&mut self, class: CorruptionClass, subject: String, detail: String) -> bool {
        if self.corruptions.len() >= MAX_CORRUPTIONS {
            self.capped = true;
            if self.limits.len() < MAX_LIMITS {
                self.limits.push(format!(
                    "integrity decode stopped after {MAX_CORRUPTIONS} corruption findings"
                ));
            }
            return false;
        }
        self.corruptions.push(IntegrityCorruption {
            class,
            subject,
            detail,
        });
        true
    }
}

fn count_at(parsed: &Value, field: &str) -> Option<usize> {
    usize::try_from(parsed.get(field)?.as_u64()?).ok()
}

fn length(value: usize) -> u64 {
    u64::try_from(value).map_or(u64::MAX, core::convert::identity)
}

#[cfg(test)]
#[path = "integrity_tests.rs"]
mod integrity_tests;
