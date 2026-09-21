//! Fjall-backed entity store: append-only observations, deduplicated snapshots, resume journal.
//!
//! Collection is interrupted constantly (politeness delays, network, operator), so every adapter
//! appends observations instead of rewriting state. The substrate is [Fjall](https://fjall-rs.github.io),
//! an embedded LSM-tree key-value store in safe Rust: writes land in a write-ahead journal and a
//! memtable, and are compacted into immutable sorted tables, so an interrupted run costs at most the
//! observations that were never flushed — never a rewritten snapshot.
//!
//! # Keyspace layout
//!
//! ```text
//! entities: <table>\0<entity-id>\0<sequence:u64 big-endian>   -> observation JSON
//! journal:  <phase>\0<key>                                    -> {key, at, payload}
//! meta:     <name>                                            -> small JSON/scalar
//! ```
//!
//! Observations are append-only: appending the same entity twice writes two rows, and
//! [`Store::consolidate`] merges them through [`Entity::merge`], which is exactly the guarantee the
//! JSONL journals used to provide. The sequence component is **big-endian** so byte order is
//! numerical order, and it is seeded from the last key present at open time, so reopening a database
//! never reuses a sequence number and never overwrites an observation.
//!
//! # Durability
//!
//! Batches are committed to the journal with [`PersistMode::SyncData`] (`fdatasync`), which is the
//! cheapest mode that survives a machine crash. [`Store::flush`] upgrades this to
//! [`PersistMode::SyncAll`] and is called at consolidation and at shutdown. A lost tail costs
//! re-running an adapter, and the resume journal is durable per completed unit of work, so a
//! resumed run does not repeat finished work.
//!
//! # Legacy journals
//!
//! Databases created before the Fjall substrate keep their rows in `<store>/entities/*.jsonl` and
//! their resume ledger in `<store>/journal/*.jsonl`. [`Store::open`] imports both exactly once
//! (recorded under `meta`), skipping the import when the marker is present, so a partially imported
//! database finishes importing on the next open without duplicating observations.

use crate::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam,
};
use anyhow::{bail, Context, Result};
use fjall::{Database, Keyspace, KeyspaceCreateOptions, PersistMode};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Hard ceiling on the observations one table may hold. A table larger than this aborts the scan
/// with a typed error instead of exhausting memory: the bound is what keeps Rule 2 (bounded control
/// flow) honest for a store whose input size is not known in advance.
pub const MAX_ROWS_PER_TABLE: u64 = 20_000_000;

/// Longest entity id the store accepts. Ids ride verbatim inside observation keys, and Fjall
/// asserts keys stay under 64 KiB; this ceiling keeps that assertion unreachable for callers.
pub const MAX_ID_BYTES: usize = 512;

/// Unified cache for the LSM tree. Bounded on purpose: the default is sized to the machine, and this
/// process is expected to share the machine with a browser and a text editor.
const CACHE_BYTES: u64 = 256 * 1024 * 1024;

const DB_DIR: &str = "fjall";
const ENTITIES: &str = "entities";
const JOURNAL: &str = "journal";
const META: &str = "meta";

/// Marker key written after a table's legacy JSONL journal has been imported.
fn imported_marker(table: Table) -> String {
    format!("imported:{}", table.file())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Table {
    Schools,
    Teams,
    Coaches,
    Athletes,
    Meets,
    Events,
    Performances,
}

impl Table {
    pub fn file(self) -> &'static str {
        match self {
            Table::Schools => "schools",
            Table::Teams => "teams",
            Table::Coaches => "coaches",
            Table::Athletes => "athletes",
            Table::Meets => "meets",
            Table::Events => "events",
            Table::Performances => "performances",
        }
    }

    pub const ALL: [Table; 7] = [
        Table::Schools,
        Table::Teams,
        Table::Coaches,
        Table::Athletes,
        Table::Meets,
        Table::Events,
        Table::Performances,
    ];

    /// Parse a wire name (`"schools"`) back into a table. Unknown names are rejected so a typo in an
    /// ingest request cannot silently create a table nobody scans.
    pub fn from_wire(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|table| table.file() == name)
    }
}

/// An entity that knows its own canonical id and how to absorb a duplicate observation.
pub trait Entity: Serialize + DeserializeOwned + Clone {
    fn entity_id(&self) -> &str;
    fn merge(&mut self, other: Self);

    /// Apply the collection contract to a merged entity. Every read of the store goes through
    /// [`Store::scan`], so a rule that lives here holds for the report, the workbook, the snapshot
    /// and the Restate handlers at once.
    fn publish(&mut self) {}

    /// How many of this entity's rows carry something the contract withheld. [`Store::consolidate`]
    /// sums this in the same pass that writes the snapshot, so reporting the count never re-scans
    /// the table.
    fn withheld_mailboxes(&self) -> usize {
        0
    }
}

/// What one [`Store::consolidate`] call produced: the rows written, and how many of them the
/// collection contract withheld a consumer mailbox from.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Consolidated {
    pub rows: usize,
    pub withheld: usize,
}

/// The id field alone, borrowed out of a serialized observation so an append can key the row without
/// deserializing the whole entity.
#[derive(Deserialize)]
struct ObservationId<'a> {
    #[serde(borrow)]
    id: &'a str,
}

/// Per-table row counts and the database's on-disk footprint. Counts are the LSM tree's own
/// estimates (`approximate_len`), which is what a status command needs without scanning millions of
/// rows.
#[derive(Debug, Clone, Serialize)]
pub struct StoreStats {
    pub tables: Vec<(String, u64)>,
    pub observations: u64,
    pub bytes_on_disk: u64,
}

pub struct Store {
    root: PathBuf,
    db: Database,
    entities: Keyspace,
    journal: Keyspace,
    meta: Keyspace,
    /// Next observation sequence per table; seeded from the last key found at open.
    sequences: BTreeMap<&'static str, AtomicU64>,
}

impl Store {
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        for sub in ["http", "out"] {
            let dir = root.join(sub);
            std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        let db = Database::builder(root.join(DB_DIR))
            .cache_size(CACHE_BYTES)
            .open()
            .with_context(|| format!("opening the Fjall database under {}", root.display()))?;
        let entities = db
            .keyspace(ENTITIES, KeyspaceCreateOptions::default)
            .context("opening the entities keyspace")?;
        let journal = db
            .keyspace(JOURNAL, KeyspaceCreateOptions::default)
            .context("opening the journal keyspace")?;
        let meta = db
            .keyspace(META, KeyspaceCreateOptions::default)
            .context("opening the meta keyspace")?;

        let mut sequences = BTreeMap::new();
        for table in Table::ALL {
            let next = Self::last_sequence(&entities, table)?
                .map(|seq| seq.saturating_add(1))
                .unwrap_or(0);
            sequences.insert(table.file(), AtomicU64::new(next));
        }

        let store = Self {
            root,
            db,
            entities,
            journal,
            meta,
            sequences,
        };
        store.import_legacy()?;
        Ok(store)
    }

    /// Highest sequence already stored for a table.
    ///
    /// Keys sort by id first and sequence second, so the *last* key in the keyspace does not carry
    /// this table's highest sequence. Resuming from it would hand out sequence numbers that are
    /// already stored for other ids, so this walks the table's prefix and takes the true maximum.
    fn last_sequence(entities: &Keyspace, table: Table) -> Result<Option<u64>> {
        let prefix = table_prefix(table);
        let mut highest: Option<u64> = None;
        for guard in entities.prefix(&prefix) {
            let key = guard.key().context("reading an entity key")?;
            let (_, sequence) = split_observation_key(&key).with_context(|| {
                format!("table {} holds a malformed observation key", table.file())
            })?;
            highest = Some(match highest {
                Some(current) => current.max(sequence),
                None => sequence,
            });
        }
        Ok(highest)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn http_cache_dir(&self) -> PathBuf {
        self.root.join("http")
    }

    pub fn out_dir(&self) -> PathBuf {
        self.root.join("out")
    }

    /// Where a pre-Fjall store kept this table's append log. Reads no longer come from here; the
    /// path survives as the one-time import source and as the materialized export location.
    pub fn table_path(&self, table: Table) -> PathBuf {
        self.root
            .join("entities")
            .join(format!("{}.jsonl", table.file()))
    }

    /// Reserve `count` consecutive observation sequences for a table.
    fn reserve(&self, table: Table, count: u64) -> Result<u64> {
        let counter = self
            .sequences
            .get(table.file())
            .context("table has no sequence counter")?;
        let start = counter.fetch_add(count, Ordering::Relaxed);
        Ok(start)
    }

    /// Append observations to a table. Each observation is its own row, exactly like the JSONL
    /// journals: merging happens at read time, so an entity seen twice keeps both evidence sets.
    ///
    /// Every record is validated before a single sequence is reserved, so a rejected batch leaves
    /// both the keyspace and the sequence counters untouched.
    pub fn append_many<T: Serialize>(&self, table: Table, records: &[T]) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }
        let mut encoded: Vec<(String, Vec<u8>)> = Vec::with_capacity(records.len());
        for record in records {
            let value = serde_json::to_vec(record).context("serializing an observation")?;
            let id = observation_id(&value)?.to_string();
            encoded.push((id, value));
        }
        let count = u64::try_from(encoded.len()).context("record count does not fit u64")?;
        let base = self.reserve(table, count)?;
        let mut batch = self.db.batch();
        for (offset, (id, value)) in encoded.into_iter().enumerate() {
            let offset = u64::try_from(offset).context("record offset does not fit u64")?;
            let sequence = base
                .checked_add(offset)
                .context("observation sequence overflowed")?;
            let key = observation_key(table, &id, sequence);
            batch.insert(&self.entities, key, value);
        }
        batch
            // `SyncData` is one `fdatasync` per batch: a crash cannot lose a completed append, and a
            // batch is a whole adapter page, not a single row.
            .durability(Some(PersistMode::SyncData))
            .commit()
            .with_context(|| {
                format!(
                    "committing {} observations to {}",
                    records.len(),
                    table.file()
                )
            })
    }

    pub fn append<T: Serialize>(&self, table: Table, record: &T) -> Result<()> {
        self.append_many(table, std::slice::from_ref(record))
    }

    /// Every observation of a table, merged into one entity per id and sorted by id.
    pub fn scan<T: Entity>(&self, table: Table) -> Result<Vec<T>> {
        let prefix = table_prefix(table);
        let mut merged: BTreeMap<String, T> = BTreeMap::new();
        let mut seen = 0_u64;
        for guard in self.entities.prefix(&prefix) {
            let raw = guard.value().context("reading an observation")?;
            let record: T = serde_json::from_slice(raw.as_ref())
                .with_context(|| format!("parsing an observation of {}", table.file()))?;
            seen = seen.saturating_add(1);
            if seen > MAX_ROWS_PER_TABLE {
                bail!(
                    "{} holds more than {MAX_ROWS_PER_TABLE} observations; consolidate with a smaller window",
                    table.file()
                );
            }
            let id = record.entity_id().to_string();
            match merged.get_mut(&id) {
                Some(existing) => existing.merge(record),
                None => {
                    merged.insert(id, record);
                }
            }
        }
        Ok(merged
            .into_values()
            .map(|mut entity| {
                entity.publish();
                entity
            })
            .collect())
    }

    /// Merge a table and write the materialized snapshot as JSONL, the read model every report and
    /// spreadsheet consumes. The withheld count comes out of the same merge pass that writes the
    /// rows, so reporting it never re-scans the table.
    pub fn consolidate<T: Entity>(&self, table: Table, out_path: &Path) -> Result<Consolidated> {
        let rows = self.scan::<T>(table)?;
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let file = std::fs::File::create(out_path)
            .with_context(|| format!("creating {}", out_path.display()))?;
        let mut writer = BufWriter::new(file);
        let mut withheld = 0_usize;
        for record in &rows {
            withheld = withheld.saturating_add(record.withheld_mailboxes());
            serde_json::to_writer(&mut writer, record)
                .with_context(|| format!("writing {}", out_path.display()))?;
            writer.write_all(b"\n")?;
        }
        writer.flush()?;
        self.flush()?;
        Ok(Consolidated {
            rows: rows.len(),
            withheld,
        })
    }

    /// Force the write-ahead journal to disk.
    /// Consolidate one table through the entity type that owns its rows. The table-to-type mapping
    /// lives here, next to the `Entity` impls, so callers stay free of a seven-arm match.
    pub fn consolidate_table(&self, table: Table, out_path: &Path) -> Result<Consolidated> {
        match table {
            Table::Schools => self.consolidate::<CanonicalSchool>(table, out_path),
            Table::Teams => self.consolidate::<CanonicalTeam>(table, out_path),
            Table::Coaches => self.consolidate::<CanonicalCoach>(table, out_path),
            Table::Athletes => self.consolidate::<CanonicalAthlete>(table, out_path),
            Table::Meets => self.consolidate::<CanonicalMeet>(table, out_path),
            Table::Events => self.consolidate::<CanonicalEvent>(table, out_path),
            Table::Performances => self.consolidate::<CanonicalPerformance>(table, out_path),
        }
    }

    pub fn flush(&self) -> Result<()> {
        self.db
            .persist(PersistMode::SyncAll)
            .context("persisting the Fjall journal")
    }

    /// Per-table observation counts and the database footprint.
    pub fn stats(&self) -> Result<StoreStats> {
        let mut tables = Vec::with_capacity(Table::ALL.len());
        let mut observations = 0_u64;
        for table in Table::ALL {
            // Per-table counts come from the sequence counters, which are exact: the next free
            // sequence equals the number of observations ever appended to that table.
            let next = self
                .sequences
                .get(table.file())
                .map(|counter| counter.load(Ordering::Relaxed))
                .unwrap_or(0);
            observations = observations.saturating_add(next);
            tables.push((table.file().to_string(), next));
        }
        Ok(StoreStats {
            tables,
            observations,
            bytes_on_disk: self.entities.disk_space(),
        })
    }

    // -- resume journal ---------------------------------------------------------------------------

    fn journal_key(phase: &str, key: &str) -> Vec<u8> {
        let mut out = Vec::with_capacity(phase.len().saturating_add(key.len()).saturating_add(2));
        out.extend_from_slice(phase.as_bytes());
        out.push(0);
        out.extend_from_slice(key.as_bytes());
        out
    }

    /// Record that a unit of work completed. Doubles as the resume ledger.
    pub fn journal_done<T: Serialize>(&self, phase: &str, key: &str, payload: &T) -> Result<()> {
        let entry = serde_json::json!({
            "key": key,
            "at": crate::net::now_iso8601(),
            "payload": payload,
        });
        let value = serde_json::to_vec(&entry).context("serializing a journal entry")?;
        let mut batch = self.db.batch();
        batch.insert(&self.journal, Self::journal_key(phase, key), value);
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .with_context(|| format!("committing journal entry {phase}:{key}"))
    }

    /// Keys already processed for a phase — the resume set.
    pub fn journal_keys(&self, phase: &str) -> Result<HashSet<String>> {
        let prefix = Self::journal_key(phase, "");
        let mut keys = HashSet::new();
        for guard in self.journal.prefix(&prefix) {
            let raw = guard.key().context("reading a journal key")?;
            let bytes: &[u8] = raw.as_ref();
            if let Some(suffix) = bytes.strip_prefix(prefix.as_slice()) {
                if let Ok(key) = std::str::from_utf8(suffix) {
                    keys.insert(key.to_string());
                }
            }
        }
        Ok(keys)
    }

    /// All journal payloads for a phase (used to rebuild adapter reports).
    pub fn journal_payloads(&self, phase: &str) -> Result<Vec<serde_json::Value>> {
        let prefix = Self::journal_key(phase, "");
        let mut out = Vec::new();
        for guard in self.journal.prefix(&prefix) {
            let raw = guard.value().context("reading a journal entry")?;
            let value: serde_json::Value = serde_json::from_slice(raw.as_ref())
                .with_context(|| format!("parsing a journal entry of {phase}"))?;
            if let Some(payload) = value.get("payload") {
                out.push(payload.clone());
            }
        }
        Ok(out)
    }

    // -- legacy import ----------------------------------------------------------------------------

    /// Import pre-Fjall journals once. A table is marked imported only after every observation of
    /// that table has been committed, so an interrupted import resumes instead of restarting.
    fn import_legacy(&self) -> Result<()> {
        for table in Table::ALL {
            let marker = imported_marker(table);
            if self
                .meta
                .contains_key(&marker)
                .context("reading the import marker")?
            {
                continue;
            }
            let path = self.table_path(table);
            if path.exists() {
                let count = self.import_observations(table, &path)?;
                tracing::info!(
                    table = table.file(),
                    observations = count,
                    "imported legacy entity journal"
                );
            }
            self.meta
                .insert(&marker, b"1".as_slice())
                .context("writing the import marker")?;
        }
        self.import_legacy_resume_journals()?;
        self.flush()
    }

    fn import_observations(&self, table: Table, path: &Path) -> Result<u64> {
        let file =
            std::fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
        let mut batch = self.db.batch();
        let mut count = 0_u64;
        let mut base = self.reserve(table, 0)?;
        for (line_no, line) in BufReader::new(file).lines().enumerate() {
            let line = line.with_context(|| format!("reading {}", path.display()))?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let line_number = line_no.saturating_add(1);
            if count >= MAX_ROWS_PER_TABLE {
                bail!(
                    "{} line {} exceeds the {MAX_ROWS_PER_TABLE} observation cap",
                    path.display(),
                    line_number
                );
            }
            let bytes = trimmed.as_bytes();
            let id = observation_id(bytes)
                .with_context(|| format!("{} line {}: no id field", path.display(), line_number))?;
            let key = observation_key(table, id, base);
            batch.insert(&self.entities, key, bytes);
            base = base.saturating_add(1);
            count = count.saturating_add(1);
        }
        self.reserve(table, count)?;
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .with_context(|| format!("importing {}", path.display()))?;
        Ok(count)
    }

    fn import_legacy_resume_journals(&self) -> Result<()> {
        let dir = self.root.join("journal");
        if !dir.exists() {
            return Ok(());
        }
        if self
            .meta
            .contains_key("imported:resume-journals")
            .context("reading the resume-journal import marker")?
        {
            return Ok(());
        }
        let entries =
            std::fs::read_dir(&dir).with_context(|| format!("listing {}", dir.display()))?;
        let mut batch = self.db.batch();
        for entry in entries {
            let entry = entry.with_context(|| format!("listing {}", dir.display()))?;
            let path = entry.path();
            let Some(phase) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            let file = std::fs::File::open(&path)
                .with_context(|| format!("opening {}", path.display()))?;
            for line in BufReader::new(file).lines() {
                let line = line.with_context(|| format!("reading {}", path.display()))?;
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) else {
                    continue;
                };
                let Some(key) = value.get("key").and_then(|key| key.as_str()) else {
                    continue;
                };
                batch.insert(
                    &self.journal,
                    Self::journal_key(phase, key),
                    trimmed.as_bytes(),
                );
            }
        }
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .context("importing legacy resume journals")?;
        self.meta
            .insert("imported:resume-journals", b"1".as_slice())
            .context("writing the resume-journal import marker")
    }
}

/// `<table>\0` — the prefix that isolates one table's observations.
fn table_prefix(table: Table) -> Vec<u8> {
    let mut out = Vec::with_capacity(table.file().len().saturating_add(1));
    out.extend_from_slice(table.file().as_bytes());
    out.push(0);
    out
}

/// `<table>\0<id>\0<sequence:u64 big-endian>`.
fn observation_key(table: Table, id: &str, sequence: u64) -> Vec<u8> {
    let mut out = table_prefix(table);
    out.extend_from_slice(id.as_bytes());
    out.push(0);
    out.extend_from_slice(&sequence.to_be_bytes());
    out
}

/// Recover `(table, sequence)` from an observation key: `<table>\0<id>\0<sequence:u64 big-endian>`.
///
/// The sequence is the fixed-width tail, so it is read positionally. Searching backwards for a NUL
/// would misparse every key whose low sequence byte is zero and leave the store unable to reopen.
fn split_observation_key(key: &[u8]) -> Option<(&str, u64)> {
    let sequence_start = key.len().checked_sub(8)?;
    let separator = sequence_start.checked_sub(1)?;
    if key.get(separator) != Some(&0) {
        return None;
    }
    let text = std::str::from_utf8(key.get(..separator)?).ok()?;
    let (table, _id) = text.split_once('\0')?;
    let sequence_bytes: [u8; 8] = key.get(sequence_start..)?.try_into().ok()?;
    Some((table, u64::from_be_bytes(sequence_bytes)))
}

/// The `id` field of a serialized observation, borrowed from the buffer that is about to be stored.
///
/// The id is carried verbatim in the key, and Fjall asserts keys stay under 64 KiB. Rejecting an
/// over-long id here keeps that assertion unreachable for any caller, including an HTTP ingest.
fn observation_id(bytes: &[u8]) -> Result<&str> {
    let parsed: ObservationId<'_> =
        serde_json::from_slice(bytes).context("observation has no string id field")?;
    if parsed.id.is_empty() {
        bail!("observation id must not be empty");
    }
    if parsed.id.len() > MAX_ID_BYTES {
        bail!(
            "observation id of {} bytes exceeds the {MAX_ID_BYTES}-byte ceiling",
            parsed.id.len()
        );
    }
    Ok(parsed.id)
}

fn union_vec<T: PartialEq + Clone>(left: &mut Vec<T>, right: &[T]) {
    for item in right {
        if !left.contains(item) {
            left.push(item.clone());
        }
    }
}

impl Entity for CanonicalSchool {
    fn entity_id(&self) -> &str {
        self.id.as_str()
    }

    fn merge(&mut self, other: Self) {
        if self.city.is_none() {
            self.city = other.city;
        }
        if self.association.is_none() {
            self.association = other.association;
        }
        if self.classification.is_none() {
            self.classification = other.classification;
        }
        if self.enrollment.is_none() {
            self.enrollment = other.enrollment;
        }
        if self.school_website.is_none() {
            self.school_website = other.school_website;
        }
        if self.athletics_website.is_none() {
            self.athletics_website = other.athletics_website;
        }
        self.co_op |= other.co_op;
        if other.name.len() > self.name.len() && self.name.starts_with(&other.name) {
            // keep the longer, more specific name
            self.name = other.name;
        }
        union_vec(&mut self.aliases, &other.aliases);
        union_vec(&mut self.source_identities, &other.source_identities);
        union_vec(&mut self.evidence, &other.evidence);
    }
}

impl Entity for CanonicalTeam {
    fn entity_id(&self) -> &str {
        self.id.as_str()
    }

    fn merge(&mut self, other: Self) {
        if self.level.is_none() {
            self.level = other.level;
        }
        union_vec(&mut self.source_identities, &other.source_identities);
        union_vec(&mut self.evidence, &other.evidence);
    }
}

impl Entity for CanonicalCoach {
    fn entity_id(&self) -> &str {
        self.id.as_str()
    }

    fn merge(&mut self, other: Self) {
        if self.professional_email.is_none() {
            self.professional_email = other.professional_email;
        }
        if self.phone.is_none() {
            self.phone = other.phone;
        }
        union_vec(&mut self.source_identities, &other.source_identities);
        union_vec(&mut self.evidence, &other.evidence);
    }

    fn publish(&mut self) {
        let Some(email) = self.professional_email.as_deref() else {
            return;
        };
        match crate::model::professional_email(email) {
            Some(published) if published == email => {}
            Some(published) => self.professional_email = Some(published),
            None => {
                // A personal mailbox never ships, whichever adapter accepted one.
                self.professional_email = None;
                self.email_withheld = true;
            }
        }
    }

    fn withheld_mailboxes(&self) -> usize {
        usize::from(self.email_withheld)
    }
}

impl Entity for CanonicalAthlete {
    fn entity_id(&self) -> &str {
        self.id.as_str()
    }

    fn merge(&mut self, other: Self) {
        union_vec(&mut self.known_names, &other.known_names);
        union_vec(&mut self.sports, &other.sports);
        union_vec(&mut self.public_profile_urls, &other.public_profile_urls);
        union_vec(&mut self.source_identities, &other.source_identities);
        union_vec(&mut self.evidence, &other.evidence);
        for observation in other.observed_grades {
            if !self.observed_grades.contains(&observation) {
                self.observed_grades.push(observation);
            }
        }
        // Any observation that disagrees with the cohort lowers confidence instead of silently
        // rewriting the athlete's graduating class.
        if self
            .observed_grades
            .iter()
            .any(|observation| observation.grad_year() != self.grad_year)
        {
            self.identity_confidence = crate::model::Confidence::LOW;
        } else if self
            .observed_grades
            .iter()
            .any(|observation| observation.grad_year() == self.grad_year)
        {
            self.identity_confidence = crate::model::Confidence::HIGH;
        }
    }
}

impl Entity for CanonicalMeet {
    fn entity_id(&self) -> &str {
        self.id.as_str()
    }

    fn merge(&mut self, other: Self) {
        if self.location.is_none() {
            self.location = other.location;
        }
        if self.end_date.is_none() {
            self.end_date = other.end_date;
        }
        if self.level == crate::model::CompetitionLevel::Unknown {
            self.level = other.level;
        }
        union_vec(&mut self.sports, &other.sports);
        union_vec(&mut self.source_identities, &other.source_identities);
        union_vec(&mut self.source_urls, &other.source_urls);
        union_vec(&mut self.evidence, &other.evidence);
    }
}

impl Entity for CanonicalEvent {
    fn entity_id(&self) -> &str {
        self.id.as_str()
    }

    fn merge(&mut self, other: Self) {
        union_vec(&mut self.source_labels, &other.source_labels);
        union_vec(&mut self.evidence, &other.evidence);
    }
}

impl Entity for CanonicalPerformance {
    fn entity_id(&self) -> &str {
        self.id.as_str()
    }

    fn merge(&mut self, other: Self) {
        if self.wind_mps.is_none() {
            self.wind_mps = other.wind_mps;
        }
        if self.place.is_none() {
            self.place = other.place;
        }
        if self.observed_grade.is_none() {
            self.observed_grade = other.observed_grade;
        }
        if self.timing.is_none() {
            self.timing = other.timing;
        }
        union_vec(&mut self.evidence, &other.evidence);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn school(name: &str) -> CanonicalSchool {
        CanonicalSchool::new("WI", name, normalize_name(name)).0
    }

    #[test]
    fn keys_with_a_zero_low_sequence_byte_still_reopen() {
        // A sequence is stored big-endian in the key tail, so every 256th key ends in 0x00 — the
        // same byte that separates the id from the sequence. Parsing that separator by search made
        // `Store::open` fail forever once such a key existed.
        let dir = tempfile::tempdir().unwrap();
        {
            let store = Store::open(dir.path()).unwrap();
            let rows: Vec<CanonicalSchool> = (0..300)
                .map(|index| school(&format!("School {index}")))
                .collect();
            store.append_many(Table::Schools, &rows).unwrap();
        }
        {
            let store = Store::open(dir.path()).unwrap();
            assert_eq!(
                store.scan::<CanonicalSchool>(Table::Schools).unwrap().len(),
                300
            );
            assert_eq!(store.stats().unwrap().observations, 300);
        }
    }

    #[test]
    fn consolidation_merges_evidence_and_identities() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let mut a = school("Abbotsford High School");
        a.source_identities
            .push(SourceIdentity::new(SourceNamespace::MilesplitTeam, "52649"));
        a.evidence.push(Evidence::parsed(
            SourceRef::id("milesplit_teams"),
            "2026-09-20",
        ));
        let mut b = a.clone();
        b.co_op = true;
        b.city = Some("Abbotsford".into());
        b.source_identities.push(SourceIdentity::new(
            SourceNamespace::AssociationSchool {
                association: "wiaa".into(),
            },
            "1",
        ));
        store.append(Table::Schools, &a).unwrap();
        store.append(Table::Schools, &b).unwrap();
        let out = dir.path().join("out/schools.jsonl");
        let count = store
            .consolidate::<CanonicalSchool>(Table::Schools, &out)
            .unwrap()
            .rows;
        assert_eq!(count, 1);
        let merged: CanonicalSchool = serde_json::from_str(
            std::fs::read_to_string(&out)
                .unwrap()
                .lines()
                .next()
                .unwrap(),
        )
        .unwrap();
        assert!(merged.co_op);
        assert_eq!(merged.city.as_deref(), Some("Abbotsford"));
        assert_eq!(merged.source_identities.len(), 2);
    }

    #[test]
    fn journal_roundtrips_resume_keys() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        store
            .journal_done(
                "milesplit_rosters",
                "wi:52649",
                &serde_json::json!({"athletes": 59}),
            )
            .unwrap();
        store
            .journal_done(
                "milesplit_rosters",
                "wi:26848",
                &serde_json::json!({"athletes": 0}),
            )
            .unwrap();
        let keys = store.journal_keys("milesplit_rosters").unwrap();
        assert!(keys.contains("wi:52649"));
        assert_eq!(keys.len(), 2);
        assert_eq!(
            store.journal_payloads("milesplit_rosters").unwrap().len(),
            2
        );
        // A second phase must not leak into the first phase's resume set.
        store
            .journal_done("other_phase", "wi:1", &serde_json::json!({}))
            .unwrap();
        assert_eq!(store.journal_keys("milesplit_rosters").unwrap().len(), 2);
    }

    #[test]
    fn observations_survive_reopen_without_overwriting() {
        let dir = tempfile::tempdir().unwrap();
        {
            let store = Store::open(dir.path()).unwrap();
            let mut first = school("Abbotsford");
            first.evidence.push(Evidence::parsed(
                SourceRef::id("wiaa_schools"),
                "2026-09-19",
            ));
            store.append(Table::Schools, &first).unwrap();
        }
        {
            // Reopening must resume the sequence, not restart it: a restarted sequence would
            // overwrite the first observation and silently drop its evidence.
            let store = Store::open(dir.path()).unwrap();
            let mut second = school("Abbotsford");
            second.evidence.push(Evidence::parsed(
                SourceRef::id("mshsl_schools"),
                "2026-09-20",
            ));
            store.append(Table::Schools, &second).unwrap();
            let rows = store.scan::<CanonicalSchool>(Table::Schools).unwrap();
            assert_eq!(rows.len(), 1);
            assert_eq!(rows.first().map(|row| row.evidence.len()), Some(2));
        }
    }

    #[test]
    fn legacy_journals_are_imported_once() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::create_dir_all(root.join("entities")).unwrap();
        std::fs::create_dir_all(root.join("journal")).unwrap();
        let row = school("Abbotsford");
        std::fs::write(
            root.join("entities/schools.jsonl"),
            format!("{}\n", serde_json::to_string(&row).unwrap()),
        )
        .unwrap();
        std::fs::write(
            root.join("journal/milesplit_rosters.jsonl"),
            "{\"key\":\"wi:1\",\"at\":\"2026-09-20\",\"payload\":{\"athletes\":5}}\n",
        )
        .unwrap();

        {
            let store = Store::open(&root).unwrap();
            assert_eq!(
                store.scan::<CanonicalSchool>(Table::Schools).unwrap().len(),
                1
            );
            assert!(store
                .journal_keys("milesplit_rosters")
                .unwrap()
                .contains("wi:1"));
        }
        // The importer is idempotent: the second open sees the marker and adds nothing.
        let store = Store::open(&root).unwrap();
        assert_eq!(
            store.scan::<CanonicalSchool>(Table::Schools).unwrap().len(),
            1
        );
        assert_eq!(store.journal_keys("milesplit_rosters").unwrap().len(), 1);
    }

    #[test]
    fn oversized_and_empty_ids_are_rejected_before_any_write() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let oversized = serde_json::json!({ "id": "s".repeat(MAX_ID_BYTES + 1) });
        assert!(store.append(Table::Schools, &oversized).is_err());
        let empty = serde_json::json!({ "id": "" });
        assert!(store.append(Table::Schools, &empty).is_err());
        assert_eq!(store.stats().unwrap().observations, 0);
    }

    #[test]
    fn stats_count_observations_per_table() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        store.append(Table::Schools, &school("Abbotsford")).unwrap();
        store
            .append_many(Table::Schools, &[school("Colby"), school("Medford")])
            .unwrap();
        let stats = store.stats().unwrap();
        let schools = stats
            .tables
            .iter()
            .find(|(table, _)| table == "schools")
            .map(|(_, count)| *count)
            .unwrap();
        assert_eq!(schools, 3);
        assert_eq!(stats.observations, 3);
    }

    #[test]
    fn a_consumer_mailbox_never_survives_a_read_but_a_school_address_does() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let school = school("Abbotsford").id;
        let mut withheld = CanonicalCoach::new(
            &school,
            "J. Riethmiller",
            Some(Sport::OutdoorTrack),
            Gender::Mixed,
            CoachRole::HeadCoach,
        );
        withheld.professional_email = Some("jriethmiller.ptc@gmail.com".to_string());
        let mut published = CanonicalCoach::new(
            &school,
            "A. Bender",
            Some(Sport::CrossCountry),
            Gender::Mixed,
            CoachRole::HeadCoach,
        );
        published.professional_email = Some("abender@ofsd.k12.wi.us".to_string());
        store
            .append_many(Table::Coaches, &[withheld, published])
            .unwrap();

        let coaches = store.scan::<CanonicalCoach>(Table::Coaches).unwrap();
        let withheld_row = coaches
            .iter()
            .find(|coach| coach.name == "J. Riethmiller")
            .unwrap();
        assert_eq!(withheld_row.professional_email, None);
        assert!(withheld_row.email_withheld);
        let published_row = coaches
            .iter()
            .find(|coach| coach.name == "A. Bender")
            .unwrap();
        assert_eq!(
            published_row.professional_email.as_deref(),
            Some("abender@ofsd.k12.wi.us")
        );
        assert!(!published_row.email_withheld);
    }
}
