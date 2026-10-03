use super::input::Inputs;
use super::{io, preserve, Result};
use census_crawl::milesplit::{OWNED_CAPTURE_PHASE, OWNED_MEET_PHASE};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam, SourceObservation,
};
use census_store::{Entity, Store, StoreError, StoreSnapshot, Table};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;

pub(super) const PROJECTION_PHASE: &str = census_crawl::milesplit::RESULT_SET_PHASE;
pub(super) const APPLICATION_PHASE: &str = "milesplit_result_set_effects_v1";
const PHASES: [&str; 4] = [
    OWNED_CAPTURE_PHASE,
    OWNED_MEET_PHASE,
    PROJECTION_PHASE,
    APPLICATION_PHASE,
];
const TABLES: [Table; 7] = [
    Table::Schools,
    Table::Meets,
    Table::Events,
    Table::Teams,
    Table::Athletes,
    Table::Performances,
    Table::SourceObservations,
];

#[derive(Serialize)]
pub(super) struct State {
    pub sequence: u64,
    pub tables: BTreeMap<String, TableState>,
    pub journals: BTreeMap<String, JournalState>,
    pub cache_originals: Value,
    pub collector_school_input_sha256: String,
}

#[derive(Serialize)]
pub(super) struct TableState {
    pub physical_rows: u64,
    pub merged_rows: u64,
    pub repeated_ids: u64,
    pub identical_content_duplicates: u64,
    pub physical_key_value_sha256: String,
    pub decoded_rows_sha256: String,
    pub content_multiset_sha256: String,
    pub content_frequencies: BTreeMap<String, u64>,
}

#[derive(Serialize)]
pub(super) struct JournalState {
    pub keys: Vec<String>,
    pub sha256: String,
}

pub(super) fn save(store: &Store, directory: &Path, inputs: &Inputs) -> Result<State> {
    io::directory(directory)?;
    let snapshot = store.snapshot();
    let tables = TABLES
        .into_iter()
        .map(|table| {
            Ok((
                table.file().into(),
                save_table(store, &snapshot, directory, table)?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    let journals = PHASES
        .into_iter()
        .map(|phase| Ok((phase.into(), save_journal(store, directory, phase)?)))
        .collect::<Result<BTreeMap<_, _>>>()?;
    let state = State {
        sequence: snapshot.sequence(),
        tables,
        journals,
        cache_originals: preserve::cache_originals(store, inputs)?,
        collector_school_input_sha256: io::digest(&io::read(
            &store.out_dir().join("schools.jsonl"),
            io::MAX_BODY,
        )?),
    };
    io::json(&directory.join("state.json"), &state)?;
    Ok(state)
}

fn save_table(
    store: &Store,
    snapshot: &StoreSnapshot<'_>,
    directory: &Path,
    table: Table,
) -> Result<TableState> {
    let walk = store.walk_table(table)?;
    if walk.rows > u64::try_from(io::MAX_ROWS)? {
        return Err(format!("{} exceeds physical row bound", table.file()).into());
    }
    let (bytes, frequencies, merged_rows) = match table {
        Table::Schools => encode::<CanonicalSchool>(snapshot, table, directory)?,
        Table::Meets => encode::<CanonicalMeet>(snapshot, table, directory)?,
        Table::Events => encode::<CanonicalEvent>(snapshot, table, directory)?,
        Table::Teams => encode::<CanonicalTeam>(snapshot, table, directory)?,
        Table::Athletes => encode::<CanonicalAthlete>(snapshot, table, directory)?,
        Table::Performances => encode::<CanonicalPerformance>(snapshot, table, directory)?,
        Table::SourceObservations => encode::<SourceObservation>(snapshot, table, directory)?,
        _ => return Err("unsupported qualification measurement table".into()),
    };
    io::write(
        &directory.join(format!("{}.physical.jsonl", table.file())),
        &bytes,
    )?;
    Ok(TableState {
        physical_rows: walk.rows,
        merged_rows,
        repeated_ids: walk.repeated_ids,
        identical_content_duplicates: walk
            .rows
            .checked_sub(u64::try_from(frequencies.len())?)
            .ok_or("row frequencies disagree")?,
        physical_key_value_sha256: snapshot.tables_digest(&[table])?,
        decoded_rows_sha256: io::digest(&bytes),
        content_multiset_sha256: io::digest(&serde_json::to_vec(&frequencies)?),
        content_frequencies: frequencies,
    })
}

type Encoded = (Vec<u8>, BTreeMap<String, u64>, u64);

fn encode<T: Entity>(
    snapshot: &StoreSnapshot<'_>,
    table: Table,
    directory: &Path,
) -> Result<Encoded> {
    let mut bytes = Vec::new();
    let mut frequencies = BTreeMap::<String, u64>::new();
    snapshot.for_each_observation::<T>(table, |row| {
        let encoded = serde_json::to_vec(&row).map_err(|source| StoreError::Json {
            detail: "encoding qualification physical row".into(),
            source,
        })?;
        append_encoded(&mut bytes, &encoded)?;
        let count = frequencies.entry(io::digest(&encoded)).or_insert(0);
        *count = count.checked_add(1).ok_or(StoreError::CounterOverflow)?;
        Ok(())
    })?;
    let mut merged = Vec::new();
    let count = snapshot.for_each_merged::<T>(table, |row| {
        let encoded = serde_json::to_vec(&row).map_err(|source| StoreError::Json {
            detail: "encoding qualification canonical row".into(),
            source,
        })?;
        append_encoded(&mut merged, &encoded)
    })?;
    io::write(
        &directory.join(format!("{}.canonical.jsonl", table.file())),
        &merged,
    )?;
    Ok((bytes, frequencies, count))
}

fn append_encoded(bytes: &mut Vec<u8>, encoded: &[u8]) -> std::result::Result<(), StoreError> {
    let extra = encoded
        .len()
        .checked_add(1)
        .ok_or(StoreError::CounterOverflow)?;
    let size = bytes
        .len()
        .checked_add(extra)
        .ok_or(StoreError::CounterOverflow)?;
    if size > io::MAX_BODY {
        return Err(StoreError::Invariant {
            detail: "qualification physical export exceeds byte bound".into(),
        });
    }
    bytes
        .try_reserve(extra)
        .map_err(|error| StoreError::Invariant {
            detail: format!("qualification export allocation failed: {error}"),
        })?;
    bytes.extend_from_slice(encoded);
    bytes.push(b'\n');
    Ok(())
}

fn save_journal(store: &Store, directory: &Path, phase: &str) -> Result<JournalState> {
    let mut keys: Vec<_> = store.journal_keys(phase)?.into_iter().collect();
    keys.sort_unstable();
    if keys.len() > io::MAX_ROWS {
        return Err(format!("{phase} exceeds journal bound").into());
    }
    let entries = keys
        .iter()
        .map(|key| -> Result<Value> {
            let payload = store
                .journal_payload(phase, key)?
                .ok_or("journal key has no payload")?;
            Ok(json!({"key": key, "payload": payload}))
        })
        .collect::<Result<Vec<_>>>()?;
    let bytes = serde_json::to_vec_pretty(&entries)?;
    io::write(&directory.join(format!("{phase}.json")), &bytes)?;
    Ok(JournalState {
        keys,
        sha256: io::digest(&bytes),
    })
}

pub(super) fn compare(before: &State, after: &State) -> Value {
    let changes: Vec<_> = before
        .tables
        .iter()
        .filter_map(|(name, old)| {
            let new = after.tables.get(name)?;
            (old.physical_key_value_sha256 != new.physical_key_value_sha256
                || old.physical_rows != new.physical_rows
                || old.content_multiset_sha256 != new.content_multiset_sha256)
                .then(|| {
                    json!({"table": name, "before": old, "after": new,
                "physical_rows_added": new.physical_rows.checked_sub(old.physical_rows)})
                })
        })
        .collect();
    let tables_equal = before.tables.keys().eq(after.tables.keys()) && changes.is_empty();
    let journals_equal = before.journals.keys().eq(after.journals.keys())
        && before.journals.iter().all(|(phase, old)| {
            after
                .journals
                .get(phase)
                .is_some_and(|new| old.keys == new.keys && old.sha256 == new.sha256)
        });
    let sequence_equal = before.sequence == after.sequence;
    let school_input_equal =
        before.collector_school_input_sha256 == after.collector_school_input_sha256;
    json!({"status": if tables_equal && journals_equal && sequence_equal && school_input_equal { "PASS" } else { "FAIL" },
        "physical_tables_unchanged": tables_equal, "journals_unchanged": journals_equal,
        "sequence_unchanged": sequence_equal, "collector_school_input_unchanged": school_input_equal,
        "before_sequence": before.sequence, "after_sequence": after.sequence, "table_changes": changes})
}
