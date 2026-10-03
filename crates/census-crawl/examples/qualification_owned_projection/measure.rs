use super::{io, Result};
use census_crawl::milesplit::{OWNED_CAPTURE_PHASE, OWNED_MEET_PHASE};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam, SourceObservation, Sport,
};
use census_store::{Entity, Store, StoreError, StoreSnapshot, Table};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;

pub(super) const PROJECTION_PHASE: &str = census_crawl::milesplit::RESULT_SET_PHASE;
const PHASES: [&str; 4] = [
    OWNED_CAPTURE_PHASE,
    OWNED_MEET_PHASE,
    PROJECTION_PHASE,
    "milesplit_result_set_effects_v1",
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
    pub meet_metadata: Value,
}

#[derive(Serialize)]
pub(super) struct TableState {
    pub physical_rows: u64,
    pub repeated_ids: u64,
    pub identical_content_duplicates: u64,
    pub content_sha256: String,
    pub content_multiset_sha256: String,
    pub physical_key_value_sha256: String,
    pub content_frequencies: BTreeMap<String, u64>,
}

#[derive(Serialize)]
pub(super) struct JournalState {
    pub keys: Vec<String>,
    pub payload_count: usize,
    pub sha256: String,
}

pub(super) fn save(store: &Store, directory: &Path) -> Result<State> {
    io::directory(directory)?;
    let snapshot = store.snapshot();
    let tables = TABLES
        .into_iter()
        .map(|table| {
            let state = save_table(store, &snapshot, directory, table)?;
            Ok((table.file().into(), state))
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
        meet_metadata: meet_metadata(&snapshot)?,
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
        return Err(format!("{} physical rows exceed qualifier bound", table.file()).into());
    }
    let (bytes, frequencies) = match table {
        Table::Schools => encode_rows::<CanonicalSchool>(snapshot, table)?,
        Table::Meets => encode_rows::<CanonicalMeet>(snapshot, table)?,
        Table::Events => encode_rows::<CanonicalEvent>(snapshot, table)?,
        Table::Teams => encode_rows::<CanonicalTeam>(snapshot, table)?,
        Table::Athletes => encode_rows::<CanonicalAthlete>(snapshot, table)?,
        Table::Performances => encode_rows::<CanonicalPerformance>(snapshot, table)?,
        Table::SourceObservations => encode_rows::<SourceObservation>(snapshot, table)?,
        _ => return Err("unowned measurement table requested".into()),
    };
    io::write(
        &directory.join(format!("{}.physical.jsonl", table.file())),
        &bytes,
    )?;
    let unique = u64::try_from(frequencies.len())?;
    Ok(TableState {
        physical_rows: walk.rows,
        repeated_ids: walk.repeated_ids,
        identical_content_duplicates: walk
            .rows
            .checked_sub(unique)
            .ok_or("row frequency mismatch")?,
        content_sha256: io::digest(&bytes),
        content_multiset_sha256: io::digest(&serde_json::to_vec(&frequencies)?),
        physical_key_value_sha256: snapshot.tables_digest(&[table])?,
        content_frequencies: frequencies,
    })
}

type EncodedRows = (Vec<u8>, BTreeMap<String, u64>);

fn encode_rows<T: Entity>(snapshot: &StoreSnapshot<'_>, table: Table) -> Result<EncodedRows> {
    let mut bytes = Vec::new();
    let mut frequencies = BTreeMap::<String, u64>::new();
    snapshot.for_each_observation::<T>(table, |row| {
        let encoded = serde_json::to_vec(&row).map_err(|source| StoreError::Json {
            detail: "encoding qualifier physical row".into(),
            source,
        })?;
        let size = bytes
            .len()
            .checked_add(encoded.len())
            .and_then(|n| n.checked_add(1))
            .ok_or(StoreError::CounterOverflow)?;
        if size > io::MAX_BODY {
            return Err(StoreError::Invariant {
                detail: "physical snapshot exceeds MAX_BODY".into(),
            });
        }
        bytes
            .try_reserve(encoded.len().saturating_add(1))
            .map_err(|error| StoreError::Invariant {
                detail: format!("physical snapshot allocation failed: {error}"),
            })?;
        let count = frequencies.entry(io::digest(&encoded)).or_insert(0);
        *count = count.checked_add(1).ok_or(StoreError::CounterOverflow)?;
        bytes.extend_from_slice(&encoded);
        bytes.push(b'\n');
        Ok(())
    })?;
    Ok((bytes, frequencies))
}

fn save_journal(store: &Store, directory: &Path, phase: &str) -> Result<JournalState> {
    let mut keys: Vec<_> = store.journal_keys(phase)?.into_iter().collect();
    keys.sort_unstable();
    if keys.len() > io::MAX_ROWS {
        return Err(format!("{phase} exceeds bounded journal count").into());
    }
    let payloads = store.journal_payloads(phase)?;
    if payloads.len() != keys.len() {
        return Err("journal key/payload snapshot mismatch".into());
    }
    let entries: Vec<_> = keys
        .iter()
        .zip(&payloads)
        .map(|(key, payload)| json!({"key": key, "payload": payload}))
        .collect();
    let bytes = serde_json::to_vec_pretty(&entries)?;
    io::write(&directory.join(format!("{phase}.json")), &bytes)?;
    Ok(JournalState {
        keys,
        payload_count: payloads.len(),
        sha256: io::digest(&bytes),
    })
}

fn meet_metadata(snapshot: &StoreSnapshot<'_>) -> Result<Value> {
    let meets: Vec<CanonicalMeet> = snapshot.scan(Table::Meets)?;
    let matching = meets
        .iter()
        .all(|meet| meet.date == "2026-03-27" && meet.sports == [Sport::OutdoorTrack]);
    Ok(json!({"retained_entities": meets.len(), "entities": meets,
        "verification_status": if meets.is_empty() { "ABSENT" }
            else if matching { "PASS" } else { "FAIL" },
        "expected_date": "2026-03-27", "expected_sport": Sport::OutdoorTrack,
        "raw_acquisition_date_role": "original raw metadata retained separately; production meet evidence uses structured acquisition date"}))
}

pub(super) fn compare(before: &State, after: &State) -> Value {
    let changes: Vec<_> = before
        .tables
        .iter()
        .filter_map(|(name, old)| {
            let new = after.tables.get(name)?;
            let changed = old.physical_rows != new.physical_rows
                || old.content_multiset_sha256 != new.content_multiset_sha256
                || old.physical_key_value_sha256 != new.physical_key_value_sha256;
            changed.then(|| {
                json!({"table": name, "before": old, "after": new,
            "physical_rows_added": new.physical_rows.checked_sub(old.physical_rows),
            "identical_content_duplicates_added": new.identical_content_duplicates
                .checked_sub(old.identical_content_duplicates)})
            })
        })
        .collect();
    let same_tables = before.tables.keys().eq(after.tables.keys()) && changes.is_empty();
    let journals_equal = before.journals.iter().all(|(phase, old)| {
        after
            .journals
            .get(phase)
            .is_some_and(|new| old.keys == new.keys && old.sha256 == new.sha256)
    }) && before.journals.len() == after.journals.len();
    json!({"status": if same_tables && journals_equal { "PASS" } else { "FAIL" },
    "physical_tables_unchanged": same_tables, "journals_unchanged": journals_equal,
    "table_changes": changes,
    "physical_duplicate_rows_observed": before.tables.iter().any(|(name, old)| {
        after.tables.get(name).is_some_and(|new| new.identical_content_duplicates > old.identical_content_duplicates)
    })})
}

pub(super) fn unresolved(state: &State) -> bool {
    state.journals.get(PROJECTION_PHASE).is_some_and(|phase| {
        phase.keys.iter().any(|key| receipt_key(key, "partial"))
            && !phase.keys.iter().any(|key| receipt_key(key, "projection"))
    })
}

fn receipt_key(key: &str, disposition: &str) -> bool {
    key.strip_prefix(disposition)
        .and_then(|key| key.strip_prefix("/725218/1266814/"))
        .is_some_and(|digest| {
            digest.len() == 64
                && digest
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
}
