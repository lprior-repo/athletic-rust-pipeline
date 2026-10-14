use super::{ObservableEntity, ReadBudget, ReadCounts, Store, StoreError, Table, TestResult};

type FixtureResult<T> = Result<T, Box<dyn std::error::Error>>;

const A1_JSON: &[u8] = br#"{"id":"alpha","value":1,"published":false}"#;
const A2_JSON: &[u8] = br#"{"id":"alpha","value":2,"published":false}"#;
const B1_JSON: &[u8] = br#"{"id":"beta","value":3,"published":false}"#;
const BAD_B_JSON: &[u8] = br#"{"id":"beta","value":"bad","published":false}"#;
const A1_KEY: &[u8] = b"schools\0alpha\0\x00\x00\x00\x00\x00\x00\x00\x00";
const A2_KEY: &[u8] = b"schools\0alpha\0\x00\x00\x00\x00\x00\x00\x00\x01";
const B1_KEY: &[u8] = b"schools\0beta\0\x00\x00\x00\x00\x00\x00\x00\x02";

fn original_rows() -> FixtureResult<[ObservableEntity; 3]> {
    let a1 = ObservableEntity {
        id: "alpha".to_owned(),
        value: 1,
        published: false,
    };
    let a2 = ObservableEntity {
        value: 2,
        ..a1.clone()
    };
    let b1 = ObservableEntity {
        id: "beta".to_owned(),
        value: 3,
        published: false,
    };
    check!(eq; serde_json::to_vec(&a1)?.as_slice(), A1_JSON);
    check!(eq; serde_json::to_vec(&a2)?.as_slice(), A2_JSON);
    check!(eq; serde_json::to_vec(&b1)?.as_slice(), B1_JSON);
    check!(eq; crate::keys::observation_key(Table::Schools, "alpha", 0).as_slice(), A1_KEY);
    check!(eq; crate::keys::observation_key(Table::Schools, "alpha", 1).as_slice(), A2_KEY);
    check!(eq; crate::keys::observation_key(Table::Schools, "beta", 2).as_slice(), B1_KEY);
    Ok([a1, a2, b1])
}

fn physical_bytes(key: &[u8], raw: &[u8]) -> FixtureResult<u64> {
    Ok(u64::try_from(
        key.len().checked_add(raw.len()).ok_or("fixture byte sum")?,
    )?)
}

fn cumulative_bytes() -> FixtureResult<[u64; 3]> {
    let first = physical_bytes(A1_KEY, A1_JSON)?;
    let second = first
        .checked_add(physical_bytes(A2_KEY, A2_JSON)?)
        .ok_or("fixture second byte coordinate")?;
    let third = second
        .checked_add(physical_bytes(B1_KEY, B1_JSON)?)
        .ok_or("fixture final byte coordinate")?;
    check!(eq; [first, second, third], [64, 128, 190]);
    Ok([first, second, third])
}

#[test]
fn raw_empty_table_accepts_zero_grants_without_invoking_callback() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let counts = store
        .snapshot()
        .for_each_observation_bounded::<ObservableEntity>(
            Table::Schools,
            ReadBudget {
                rows: 0,
                bytes: 0,
                record_bytes: 0,
            },
            |_, _| {
                Err(StoreError::Refused {
                    detail: "empty table callback must not run".to_owned(),
                })
            },
        )?;
    check!(eq; counts, ReadCounts::default());
    Ok(())
}

#[test]
fn raw_zero_row_grant_refuses_before_malformed_json_decode() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    store.entities.insert(A1_KEY, b"{")?;
    let result = store
        .snapshot()
        .for_each_observation_bounded::<ObservableEntity>(
            Table::Schools,
            ReadBudget {
                rows: 0,
                bytes: 0,
                record_bytes: 0,
            },
            |_, _| {
                Err(StoreError::Refused {
                    detail: "zero row callback must not run".to_owned(),
                })
            },
        );
    check!(matches!(
        result,
        Err(StoreError::BudgetExceeded {
            what: "capture physical rows",
            needed: 1,
            available: 0,
        })
    ));
    Ok(())
}

#[test]
fn raw_record_grant_accepts_exact_original_json_length() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let [a1, _, _] = original_rows()?;
    let exact = physical_bytes(A1_KEY, A1_JSON)?;
    store.append(Table::Schools, &a1)?;
    let mut visited = Vec::new();
    let counts = store.snapshot().for_each_observation_bounded(
        Table::Schools,
        ReadBudget {
            rows: 1,
            bytes: exact,
            record_bytes: A1_JSON.len(),
        },
        |row: ObservableEntity, bytes| {
            visited.push((row, bytes));
            Ok(())
        },
    )?;
    check!(eq; visited, vec![(a1, exact)]);
    check!(eq; counts, ReadCounts { key_visits: 1, decoded_rows: 1, encoded_bytes: exact, merged_rows: 0 });
    Ok(())
}

#[test]
fn raw_record_grant_refuses_one_byte_short_before_callback() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let [a1, _, _] = original_rows()?;
    let exact = physical_bytes(A1_KEY, A1_JSON)?;
    let short = A1_JSON.len().checked_sub(1).ok_or("fixture record size")?;
    store.append(Table::Schools, &a1)?;
    let result = store
        .snapshot()
        .for_each_observation_bounded::<ObservableEntity>(
            Table::Schools,
            ReadBudget {
                rows: 1,
                bytes: exact,
                record_bytes: short,
            },
            |_, _| {
                Err(StoreError::Refused {
                    detail: "short record callback must not run".to_owned(),
                })
            },
        );
    check!(matches!(result, Err(StoreError::RowTooLarge { bytes, max })
        if bytes == A1_JSON.len() && max == short));
    Ok(())
}

#[test]
fn raw_record_refusal_precedes_total_encoded_byte_refusal() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let [a1, _, _] = original_rows()?;
    let short = A1_JSON.len().checked_sub(1).ok_or("fixture record size")?;
    store.append(Table::Schools, &a1)?;
    let result = store
        .snapshot()
        .for_each_observation_bounded::<ObservableEntity>(
            Table::Schools,
            ReadBudget {
                rows: 1,
                bytes: 0,
                record_bytes: short,
            },
            |_, _| {
                Err(StoreError::Refused {
                    detail: "record refusal callback must not run".to_owned(),
                })
            },
        );
    check!(matches!(result, Err(StoreError::RowTooLarge { bytes, max })
        if bytes == A1_JSON.len() && max == short));
    Ok(())
}

#[test]
fn raw_late_decode_failure_retains_only_provisional_callback() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let [a1, _, _] = original_rows()?;
    let first = physical_bytes(A1_KEY, A1_JSON)?;
    let total = first
        .checked_add(physical_bytes(B1_KEY, BAD_B_JSON)?)
        .ok_or("fixture corrupt byte sum")?;
    store.append(Table::Schools, &a1)?;
    store.entities.insert(B1_KEY, BAD_B_JSON)?;
    let mut provisional = Vec::new();
    let result = store.snapshot().for_each_observation_bounded(
        Table::Schools,
        ReadBudget {
            rows: 2,
            bytes: total,
            record_bytes: BAD_B_JSON.len(),
        },
        |row: ObservableEntity, bytes| {
            provisional.push((row, bytes));
            Ok(())
        },
    );
    check!(eq; provisional, vec![(a1, first)]);
    check!(matches!(result, Err(StoreError::Decode { key, source })
        if key == "schools:beta#2" && source.classify() == serde_json::error::Category::Data));
    drop(provisional);
    Ok(())
}

#[test]
fn raw_late_payload_identity_mismatch_preserves_exact_invariant() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let [a1, _, _] = original_rows()?;
    let first = physical_bytes(A1_KEY, A1_JSON)?;
    let total = first
        .checked_add(physical_bytes(B1_KEY, A2_JSON)?)
        .ok_or("fixture mismatched byte sum")?;
    store.append(Table::Schools, &a1)?;
    store.entities.insert(B1_KEY, A2_JSON)?;
    let mut provisional = Vec::new();
    let result = store.snapshot().for_each_observation_bounded(
        Table::Schools,
        ReadBudget {
            rows: 2,
            bytes: total,
            record_bytes: A2_JSON.len(),
        },
        |row: ObservableEntity, bytes| {
            provisional.push((row, bytes));
            Ok(())
        },
    );
    check!(eq; provisional, vec![(a1, first)]);
    check!(matches!(result, Err(StoreError::Invariant { detail })
        if detail == "key schools:beta#2 disagrees with its payload identity"));
    drop(provisional);
    Ok(())
}

#[test]
fn old_and_fresh_raw_snapshots_return_exact_physical_multisets() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let [a1, a2, b1] = original_rows()?;
    let [first, second, total] = cumulative_bytes()?;
    store.append(Table::Schools, &a1)?;
    let old = store.snapshot();
    store.append_many(Table::Schools, &[a2.clone(), b1.clone()])?;
    let fresh = store.snapshot();
    let mut old_rows = Vec::new();
    let old_counts = old.for_each_observation_bounded(
        Table::Schools,
        ReadBudget {
            rows: 1,
            bytes: first,
            record_bytes: A1_JSON.len(),
        },
        |row: ObservableEntity, bytes| {
            old_rows.push((row, bytes));
            Ok(())
        },
    )?;
    let mut fresh_rows = Vec::new();
    let fresh_counts = fresh.for_each_observation_bounded(
        Table::Schools,
        ReadBudget {
            rows: 3,
            bytes: total,
            record_bytes: A1_JSON.len(),
        },
        |row: ObservableEntity, bytes| {
            fresh_rows.push((row, bytes));
            Ok(())
        },
    )?;
    check!(eq; old_rows, vec![(a1.clone(), first)]);
    check!(eq; fresh_rows, vec![(a1, first), (a2, second), (b1, total)]);
    check!(eq; old_counts, ReadCounts { key_visits: 1, decoded_rows: 1, encoded_bytes: first, merged_rows: 0 });
    check!(eq; fresh_counts, ReadCounts { key_visits: 3, decoded_rows: 3, encoded_bytes: total, merged_rows: 0 });
    check!(old.sequence() < fresh.sequence());
    Ok(())
}

#[test]
fn merged_callbacks_use_transition_and_final_cumulative_byte_coordinates() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let rows = original_rows()?;
    let [_, a2, b1] = rows.clone();
    let [_, _, total] = cumulative_bytes()?;
    store.append_many(Table::Schools, &rows)?;
    let mut published = Vec::new();
    let counts = store.snapshot().for_each_merged_bounded(
        Table::Schools,
        ReadBudget {
            rows: 3,
            bytes: total,
            record_bytes: A1_JSON.len(),
        },
        |row: ObservableEntity, bytes| {
            published.push((row, bytes));
            Ok(())
        },
    )?;
    check!(eq; published, vec![
        (ObservableEntity { published: true, ..a2 }, total),
        (ObservableEntity { published: true, ..b1 }, total),
    ]);
    check!(eq; counts, ReadCounts { key_visits: 3, decoded_rows: 3, encoded_bytes: total, merged_rows: 2 });
    Ok(())
}

#[test]
fn merged_transition_callback_refusal_preserves_exact_error_and_stops_publication() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let rows = original_rows()?;
    let [_, a2, _] = rows.clone();
    let [_, _, total] = cumulative_bytes()?;
    store.append_many(Table::Schools, &rows)?;
    let mut published = Vec::new();
    let result = store.snapshot().for_each_merged_bounded(
        Table::Schools,
        ReadBudget {
            rows: 3,
            bytes: total,
            record_bytes: A1_JSON.len(),
        },
        |row: ObservableEntity, bytes| {
            published.push((row, bytes));
            Err(StoreError::BudgetExceeded {
                what: "fixture transition callback",
                needed: 37,
                available: 11,
            })
        },
    );
    check!(eq; published, vec![(ObservableEntity { published: true, ..a2 }, total)]);
    check!(matches!(
        result,
        Err(StoreError::BudgetExceeded {
            what: "fixture transition callback",
            needed: 37,
            available: 11,
        })
    ));
    Ok(())
}

#[test]
fn merged_final_callback_refusal_preserves_exact_error_without_terminal_counts() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let rows = original_rows()?;
    let [_, a2, b1] = rows.clone();
    let [_, _, total] = cumulative_bytes()?;
    store.append_many(Table::Schools, &rows)?;
    let mut published = Vec::new();
    let result = store.snapshot().for_each_merged_bounded(
        Table::Schools,
        ReadBudget {
            rows: 3,
            bytes: total,
            record_bytes: A1_JSON.len(),
        },
        |row: ObservableEntity, bytes| {
            let final_entity = row.id == "beta";
            published.push((row, bytes));
            if final_entity {
                return Err(StoreError::BudgetExceeded {
                    what: "fixture final callback",
                    needed: 19,
                    available: 7,
                });
            }
            Ok(())
        },
    );
    check!(eq; published, vec![
        (ObservableEntity { published: true, ..a2 }, total),
        (ObservableEntity { published: true, ..b1 }, total),
    ]);
    check!(matches!(
        result,
        Err(StoreError::BudgetExceeded {
            what: "fixture final callback",
            needed: 19,
            available: 7,
        })
    ));
    Ok(())
}
