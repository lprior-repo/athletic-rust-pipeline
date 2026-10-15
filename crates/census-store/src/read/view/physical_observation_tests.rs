use super::{decode_physical_observation, ReadBudget, ReadCounts};
use crate::{Entity, Store, StoreError, Table};
use serde::{Deserialize, Serialize};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
type PhysicalRows = Vec<(PhysicalEntity, Vec<u8>, Vec<u8>, u64)>;

const A1_KEY: &[u8] = b"schools\0alpha\0\x00\x00\x00\x00\x00\x00\x00\x00";
const A2_KEY: &[u8] = b"schools\0alpha\0\x00\x00\x00\x00\x00\x00\x00\x01";
const B1_KEY: &[u8] = b"schools\0beta\0\x00\x00\x00\x00\x00\x00\x00\x02";
const A1_RAW: &[u8] = br#" { "value": 1, "id": "alpha", "legacy": true } "#;
const A2_RAW: &[u8] = br#"{"id":"alpha","value":2}"#;
const B1_RAW: &[u8] = br#"{"value":3,"id":"beta"}"#;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PhysicalEntity {
    id: String,
    value: u8,
    #[serde(default)]
    published: bool,
}

impl Entity for PhysicalEntity {
    fn entity_id(&self) -> &str {
        &self.id
    }

    fn merge(&mut self, other: Self) {
        self.value = other.value;
    }

    fn publish(&mut self) {
        self.published = true;
    }
}

fn entity(id: &str, value: u8) -> PhysicalEntity {
    PhysicalEntity {
        id: id.into(),
        value,
        published: false,
    }
}

fn encoded_bytes(key: &[u8], raw: &[u8]) -> TestResult<u64> {
    Ok(u64::try_from(
        key.len()
            .checked_add(raw.len())
            .ok_or("fixture byte overflow")?,
    )?)
}

fn limits(rows: u64, bytes: u64, record_bytes: usize) -> ReadBudget {
    ReadBudget {
        rows,
        bytes,
        record_bytes,
    }
}

fn seed(store: &Store) -> TestResult {
    for (key, raw) in [(A1_KEY, A1_RAW), (A2_KEY, A2_RAW), (B1_KEY, B1_RAW)] {
        store.entities.insert(key, raw)?;
    }
    Ok(())
}

#[test]
fn physical_visitor_preserves_original_wire_bytes_versions_and_unpublished_values() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    seed(&store)?;
    let first_bytes = encoded_bytes(A1_KEY, A1_RAW)?;
    let second_bytes = first_bytes
        .checked_add(encoded_bytes(A2_KEY, A2_RAW)?)
        .ok_or("fixture bytes")?;
    let total = second_bytes
        .checked_add(encoded_bytes(B1_KEY, B1_RAW)?)
        .ok_or("fixture bytes")?;
    let snapshot = store.snapshot();
    let mut physical = PhysicalRows::new();
    let counts = snapshot.for_each_physical_observation_bounded(
        Table::Schools,
        limits(3, total, A1_RAW.len()),
        |row: PhysicalEntity, key, raw, bytes| {
            physical.push((row, key.to_vec(), raw.to_vec(), bytes));
            Ok(())
        },
    )?;
    check!(eq; physical, vec![
        (entity("alpha", 1), A1_KEY.to_vec(), A1_RAW.to_vec(), first_bytes),
        (entity("alpha", 2), A2_KEY.to_vec(), A2_RAW.to_vec(), second_bytes),
        (entity("beta", 3), B1_KEY.to_vec(), B1_RAW.to_vec(), total),
    ]);
    check!(eq; counts, ReadCounts { key_visits: 3, decoded_rows: 3, encoded_bytes: total, merged_rows: 0 });
    check!(ne; serde_json::to_vec(&entity("alpha", 1))?.as_slice(), A1_RAW);
    let mut decoded = Vec::new();
    let decoded_counts = snapshot.for_each_observation_bounded(
        Table::Schools,
        limits(3, total, A1_RAW.len()),
        |row: PhysicalEntity, bytes| {
            decoded.push((row, bytes));
            Ok(())
        },
    )?;
    check!(eq; decoded_counts, counts);
    check!(eq; decoded, physical.into_iter().map(|(row, _, _, bytes)| (row, bytes)).collect::<Vec<_>>());
    let mut merged = Vec::new();
    let merged_counts = snapshot.for_each_merged_bounded(
        Table::Schools,
        limits(3, total, A1_RAW.len()),
        |row: PhysicalEntity, bytes| {
            merged.push((row, bytes));
            Ok(())
        },
    )?;
    check!(eq; merged, vec![
        (PhysicalEntity { published: true, ..entity("alpha", 2) }, total),
        (PhysicalEntity { published: true, ..entity("beta", 3) }, total),
    ]);
    check!(eq; merged_counts, ReadCounts { merged_rows: 2, ..counts });
    Ok(())
}

#[test]
fn physical_visitor_keeps_pinned_key_value_and_size_after_live_replacement() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    store.entities.insert(A1_KEY, A1_RAW)?;
    let held = store.snapshot();
    let exact = encoded_bytes(A1_KEY, A1_RAW)?;
    let oversized = vec![b'!'; A1_RAW.len().checked_add(1).ok_or("fixture size")?];
    store.entities.insert(A1_KEY, oversized.as_slice())?;
    store.entities.insert(B1_KEY, B1_RAW)?;
    let mut visited = PhysicalRows::new();
    let counts = held.for_each_physical_observation_bounded(
        Table::Schools,
        limits(1, exact, A1_RAW.len()),
        |row: PhysicalEntity, key, raw, bytes| {
            visited.push((row, key.to_vec(), raw.to_vec(), bytes));
            Ok(())
        },
    )?;
    check!(eq; visited, vec![(entity("alpha", 1), A1_KEY.to_vec(), A1_RAW.to_vec(), exact)]);
    check!(eq; counts, ReadCounts { key_visits: 1, decoded_rows: 1, encoded_bytes: exact, merged_rows: 0 });
    let mut callbacks = 0;
    let result = store
        .snapshot()
        .for_each_physical_observation_bounded::<PhysicalEntity>(
            Table::Schools,
            limits(1, exact, A1_RAW.len()),
            |_, _, _, _| {
                callbacks += 1;
                Ok(())
            },
        );
    check!(
        matches!(result, Err(StoreError::RowTooLarge { bytes, max }) if bytes == oversized.len() && max == A1_RAW.len())
    );
    check!(eq; callbacks, 0);
    Ok(())
}

#[test]
fn physical_visitor_charges_original_key_and_value_before_callback() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    store.entities.insert(A1_KEY, A1_RAW)?;
    let exact = encoded_bytes(A1_KEY, A1_RAW)?;
    for available in [
        u64::try_from(A1_RAW.len())?,
        exact.checked_sub(1).ok_or("fixture size")?,
    ] {
        let mut callbacks = 0;
        let result = store
            .snapshot()
            .for_each_physical_observation_bounded::<PhysicalEntity>(
                Table::Schools,
                limits(1, available, A1_RAW.len()),
                |_, _, _, _| {
                    callbacks += 1;
                    Ok(())
                },
            );
        check!(
            matches!(result, Err(StoreError::BudgetExceeded { what: "capture encoded bytes", needed, available: found }) if needed == exact && found == available)
        );
        check!(eq; callbacks, 0);
    }
    let mut visited = PhysicalRows::new();
    let counts = store
        .snapshot()
        .for_each_physical_observation_bounded::<PhysicalEntity>(
            Table::Schools,
            limits(1, exact, A1_RAW.len()),
            |row, key, raw, bytes| {
                visited.push((row, key.to_vec(), raw.to_vec(), bytes));
                Ok(())
            },
        )?;
    check!(eq; visited, vec![(entity("alpha", 1), A1_KEY.to_vec(), A1_RAW.to_vec(), exact)]);
    check!(eq; counts, ReadCounts { key_visits: 1, decoded_rows: 1, encoded_bytes: exact, merged_rows: 0 });
    Ok(())
}

#[test]
fn physical_preflight_preserves_row_then_record_then_total_then_decode_error_order() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let malformed = b"{!";
    store.entities.insert(A1_KEY, malformed.as_slice())?;
    let exact = encoded_bytes(A1_KEY, malformed)?;
    let mut callbacks = 0;
    let snapshot = store.snapshot();
    let result = snapshot.for_each_physical_observation_bounded::<PhysicalEntity>(
        Table::Schools,
        limits(0, 0, 0),
        |_, _, _, _| {
            callbacks += 1;
            Ok(())
        },
    );
    check!(matches!(
        result,
        Err(StoreError::BudgetExceeded {
            what: "capture physical rows",
            needed: 1,
            available: 0
        })
    ));
    let result = snapshot.for_each_physical_observation_bounded::<PhysicalEntity>(
        Table::Schools,
        limits(1, 0, 1),
        |_, _, _, _| {
            callbacks += 1;
            Ok(())
        },
    );
    check!(matches!(
        result,
        Err(StoreError::RowTooLarge { bytes: 2, max: 1 })
    ));
    let result = snapshot.for_each_physical_observation_bounded::<PhysicalEntity>(
        Table::Schools,
        limits(1, 0, 2),
        |_, _, _, _| {
            callbacks += 1;
            Ok(())
        },
    );
    check!(
        matches!(result, Err(StoreError::BudgetExceeded { what: "capture encoded bytes", needed, available: 0 }) if needed == exact)
    );
    let result = snapshot.for_each_physical_observation_bounded::<PhysicalEntity>(
        Table::Schools,
        limits(1, exact, 2),
        |_, _, _, _| {
            callbacks += 1;
            Ok(())
        },
    );
    check!(
        matches!(result, Err(StoreError::Decode { key, source }) if key == "schools:alpha#0" && source.classify() == serde_json::error::Category::Syntax)
    );
    check!(eq; callbacks, 0);
    Ok(())
}

#[test]
fn physical_callback_refusal_stops_before_later_corruption_and_preserves_exact_error() -> TestResult
{
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    seed(&store)?;
    store.entities.insert(B1_KEY, b"{")?;
    let mut visited = Vec::new();
    let result = store.snapshot().for_each_physical_observation_bounded(
        Table::Schools,
        limits(3, 4096, 4096),
        |row: PhysicalEntity, key, raw, _| {
            visited.push((row, key.to_vec(), raw.to_vec()));
            if key == A2_KEY {
                return Err(StoreError::Refused {
                    detail: "synthetic frozen sink refused second row".into(),
                });
            }
            Ok(())
        },
    );
    check!(
        matches!(result, Err(StoreError::Refused { detail }) if detail == "synthetic frozen sink refused second row")
    );
    check!(eq; visited, vec![
        (entity("alpha", 1), A1_KEY.to_vec(), A1_RAW.to_vec()),
        (entity("alpha", 2), A2_KEY.to_vec(), A2_RAW.to_vec()),
    ]);
    Ok(())
}

#[test]
fn physical_late_decode_failure_does_not_return_successful_partial_counts() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    store.entities.insert(A1_KEY, A1_RAW)?;
    store.entities.insert(B1_KEY, b"{")?;
    let mut visited = Vec::new();
    let result = store.snapshot().for_each_physical_observation_bounded(
        Table::Schools,
        limits(2, 4096, 4096),
        |row: PhysicalEntity, key, raw, _| {
            visited.push((row, key.to_vec(), raw.to_vec()));
            Ok(())
        },
    );
    check!(
        matches!(result, Err(StoreError::Decode { key, source }) if key == "schools:beta#2" && source.classify() == serde_json::error::Category::Eof)
    );
    check!(eq; visited, vec![(entity("alpha", 1), A1_KEY.to_vec(), A1_RAW.to_vec())]);
    Ok(())
}

#[test]
fn empty_physical_table_accepts_zero_grants_without_a_callback() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let mut callbacks = 0;
    let counts = store
        .snapshot()
        .for_each_physical_observation_bounded::<PhysicalEntity>(
            Table::Schools,
            limits(0, 0, 0),
            |_, _, _, _| {
                callbacks += 1;
                Ok(())
            },
        )?;
    check!(eq; counts, ReadCounts::default());
    check!(eq; callbacks, 0);
    Ok(())
}

#[test]
fn replay_decoder_shares_original_table_key_and_payload_identity_checks_without_publish(
) -> TestResult {
    let decoded: PhysicalEntity =
        crate::decode_physical_observation(Table::Schools, A1_KEY, A1_RAW)?;
    check!(eq; decoded, entity("alpha", 1));
    let through_read: PhysicalEntity =
        crate::read::decode_physical_observation(Table::Schools, A1_KEY, A1_RAW)?;
    check!(eq; through_read, decoded);
    let wrong_table = decode_physical_observation::<PhysicalEntity>(Table::Teams, A1_KEY, b"{");
    check!(
        matches!(wrong_table, Err(StoreError::Invariant { detail }) if detail == "observation key schools:alpha#0 does not identify a row of teams")
    );
    let mismatched = decode_physical_observation::<PhysicalEntity>(Table::Schools, A1_KEY, B1_RAW);
    check!(
        matches!(mismatched, Err(StoreError::Invariant { detail }) if detail == "key schools:alpha#0 disagrees with its payload identity")
    );
    let malformed = decode_physical_observation::<PhysicalEntity>(Table::Schools, b"bad-key", b"{");
    check!(
        matches!(malformed, Err(StoreError::Invariant { detail }) if detail == "malformed observation key bad-key")
    );
    let empty_key = crate::keys::observation_key(Table::Schools, "", 0);
    let empty = decode_physical_observation::<PhysicalEntity>(Table::Schools, &empty_key, b"{");
    check!(
        matches!(empty, Err(StoreError::Invariant { detail }) if detail == "key schools:#0 identifies no row id")
    );
    Ok(())
}

#[test]
fn physical_derived_visits_preserve_current_generation_keys_and_replay_uses_same_decoder(
) -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let generation = store.snapshot().derived_generation();
    let current = crate::keys::derived_key(Table::ReviewCases, generation, b"alpha");
    let other_generation = generation.checked_add(1).ok_or("fixture generation")?;
    let other = crate::keys::derived_key(Table::ReviewCases, other_generation, b"alpha");
    store.entities.insert(current.as_slice(), A1_RAW)?;
    store.entities.insert(other.as_slice(), A2_RAW)?;
    let exact = encoded_bytes(&current, A1_RAW)?;
    let mut visited = PhysicalRows::new();
    let counts = store.snapshot().for_each_physical_observation_bounded(
        Table::ReviewCases,
        limits(1, exact, A1_RAW.len()),
        |row: PhysicalEntity, key, raw, bytes| {
            visited.push((row, key.to_vec(), raw.to_vec(), bytes));
            Ok(())
        },
    )?;
    check!(eq; visited, vec![(entity("alpha", 1), current.clone(), A1_RAW.to_vec(), exact)]);
    check!(eq; counts, ReadCounts { key_visits: 1, decoded_rows: 1, encoded_bytes: exact, merged_rows: 0 });
    let decoded: PhysicalEntity =
        decode_physical_observation(Table::ReviewCases, &current, A1_RAW)?;
    check!(eq; decoded, entity("alpha", 1));
    let wrong_table =
        decode_physical_observation::<PhysicalEntity>(Table::Coverage, &current, b"{");
    check!(
        matches!(wrong_table, Err(StoreError::Invariant { detail }) if detail == format!("malformed derived key derived:review_cases:{generation}#alpha"))
    );
    Ok(())
}
