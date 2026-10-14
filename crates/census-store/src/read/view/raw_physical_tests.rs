use super::*;
use crate::{Entity, Store};
use serde::{Deserialize, Serialize};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ObservableEntity {
    id: String,
    value: u8,
    published: bool,
}

impl Entity for ObservableEntity {
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

#[test]
fn raw_visits_preserve_duplicates_and_never_merge_or_publish() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let original = ObservableEntity {
        id: "fixture".to_owned(),
        value: 1,
        published: false,
    };
    let changed = ObservableEntity {
        value: 2,
        ..original.clone()
    };
    store.append_many(
        Table::Schools,
        &[original.clone(), original.clone(), changed.clone()],
    )?;
    let snapshot = store.snapshot();
    let mut raw = Vec::new();
    let counts = snapshot.for_each_observation_bounded(
        Table::Schools,
        ReadBudget {
            rows: 3,
            bytes: 4096,
            record_bytes: 4096,
        },
        |row: ObservableEntity, _| {
            raw.push(row);
            Ok(())
        },
    )?;
    check!(eq; raw, [original.clone(), original, changed.clone()]);
    check!(eq; counts.decoded_rows, 3);
    check!(eq; counts.merged_rows, 0);
    let merged = snapshot.scan::<ObservableEntity>(Table::Schools)?;
    check!(eq; merged, [ObservableEntity { published: true, ..changed }]);
    Ok(())
}

#[test]
fn physical_byte_grant_counts_known_key_and_json_before_callback() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let row = ObservableEntity {
        id: "fixture".to_owned(),
        value: 1,
        published: false,
    };
    let encoded = serde_json::to_vec(&row)?;
    let key_bytes = Table::Schools
        .file()
        .len()
        .checked_add(1)
        .and_then(|n| n.checked_add("fixture".len()))
        .and_then(|n| n.checked_add(1 + 8))
        .ok_or("test key size")?;
    let exact = u64::try_from(
        key_bytes
            .checked_add(encoded.len())
            .ok_or("test total size")?,
    )?;
    store.append(Table::Schools, &row)?;
    let snapshot = store.snapshot();
    let lower = exact.checked_sub(1).ok_or("test lower size")?;
    let mut calls = 0;
    let result = snapshot.for_each_observation_bounded::<ObservableEntity>(
        Table::Schools,
        ReadBudget {
            rows: 1,
            bytes: lower,
            record_bytes: encoded.len(),
        },
        |_, _| {
            calls += 1;
            Ok(())
        },
    );
    check!(
        matches!(result, Err(StoreError::BudgetExceeded { what: "capture encoded bytes", needed, available }) if needed == exact && available == lower)
    );
    check!(eq; calls, 0);
    let mut visited = None;
    let counts = snapshot.for_each_observation_bounded(
        Table::Schools,
        ReadBudget {
            rows: 1,
            bytes: exact,
            record_bytes: encoded.len(),
        },
        |actual: ObservableEntity, bytes| {
            visited = Some((actual, bytes));
            Ok(())
        },
    )?;
    check!(eq; visited, Some((row, exact)));
    check!(eq; counts, ReadCounts { key_visits: 1, decoded_rows: 1, encoded_bytes: exact, merged_rows: 0 });
    Ok(())
}
