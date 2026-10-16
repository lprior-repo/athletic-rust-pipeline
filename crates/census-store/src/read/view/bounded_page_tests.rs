use super::{PhysicalObservationCursor, PhysicalObservationPage};
use crate::{Entity, ReadBudget, Store, StoreError, StoreSnapshot, Table};
use serde::{Deserialize, Serialize};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
type Rows = Vec<(Row, Vec<u8>, Vec<u8>)>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Row {
    id: String,
    value: u8,
}

impl Entity for Row {
    fn entity_id(&self) -> &str {
        &self.id
    }

    fn merge(&mut self, other: Self) {
        self.value = other.value;
    }
}

const RAW: [&[u8]; 3] = [
    br#" { "id": "alpha", "value": 1, "legacy": true } "#,
    br#"{"id":"alpha","value":2}"#,
    br#"{"id":"beta","value":3}"#,
];

fn budget(rows: u64, bytes: u64) -> ReadBudget {
    ReadBudget {
        rows,
        bytes,
        record_bytes: 1024,
    }
}

fn seed(store: &Store) -> TestResult {
    for (id, sequence, raw) in [
        ("alpha", 0, RAW[0]),
        ("alpha", 1, RAW[1]),
        ("beta", 2, RAW[2]),
    ] {
        store.entities.insert(
            crate::keys::observation_key(Table::Schools, id, sequence),
            raw,
        )?;
    }
    Ok(())
}

fn page<'a>(
    snapshot: &'a StoreSnapshot<'_>,
    budget: ReadBudget,
    cursor: Option<&PhysicalObservationCursor<'_>>,
) -> TestResult<(PhysicalObservationPage<'a>, Rows)> {
    let mut rows = Vec::new();
    let page = snapshot.for_each_physical_observation_page(
        Table::Schools,
        budget,
        cursor,
        |row: Row, key, raw, _| {
            rows.push((row, key.to_vec(), raw.to_vec()));
            Ok(())
        },
    )?;
    Ok((page, rows))
}

fn continuation(page: PhysicalObservationPage<'_>) -> TestResult<PhysicalObservationCursor<'_>> {
    match page {
        PhysicalObservationPage::Continue { cursor, .. } => Ok(cursor),
        PhysicalObservationPage::Exhausted { .. } => Err("expected continuation".into()),
    }
}

#[test]
fn one_row_pages_preserve_duplicate_variants_and_match_whole_scan() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    seed(&store)?;
    let snapshot = store.snapshot();
    let (first, mut rows) = page(&snapshot, budget(1, 1024), None)?;
    check!(eq; rows.len(), 1);
    let first = continuation(first)?;
    let (second, more) = page(&snapshot, budget(1, 1024), Some(&first))?;
    rows.extend(more);
    check!(eq; rows[0].0.id, rows[1].0.id);
    check!(ne; rows[0].1, rows[1].1);
    let second = continuation(second)?;
    let (last, more) = page(&snapshot, budget(1, 1024), Some(&second))?;
    rows.extend(more);
    check!(
        matches!(last, PhysicalObservationPage::Exhausted { counts } if counts.decoded_rows == 1 && counts.key_visits == 1)
    );
    let mut whole = Vec::new();
    snapshot.for_each_physical_observation_bounded(
        Table::Schools,
        budget(3, 4096),
        |row: Row, key, raw, _| {
            whole.push((row, key.to_vec(), raw.to_vec()));
            Ok(())
        },
    )?;
    check!(eq; rows, whole);
    check!(eq; rows.iter().map(|row| row.2.as_slice()).collect::<Vec<_>>(), RAW);
    Ok(())
}

#[test]
fn exact_row_and_byte_boundaries_report_exhaustion_without_an_extra_page() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    seed(&store)?;
    let snapshot = store.snapshot();
    let total = snapshot.for_each_physical_observation_bounded::<Row>(
        Table::Schools,
        budget(3, 4096),
        |_, _, _, _| Ok(()),
    )?;
    let (outcome, rows) = page(&snapshot, budget(3, total.encoded_bytes), None)?;
    check!(eq; rows.len(), 3);
    check!(matches!(outcome, PhysicalObservationPage::Exhausted { counts } if counts == total));
    Ok(())
}

#[test]
fn byte_limited_page_resumes_at_the_unconsumed_physical_row() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    seed(&store)?;
    let snapshot = store.snapshot();
    let first_bytes = u64::try_from(
        crate::keys::observation_key(Table::Schools, "alpha", 0)
            .len()
            .checked_add(RAW[0].len())
            .ok_or("fixture size")?,
    )?;
    let (first, rows) = page(&snapshot, budget(3, first_bytes), None)?;
    check!(eq; rows.len(), 1);
    match &first {
        PhysicalObservationPage::Continue { counts, .. } => {
            check!(eq; counts.encoded_bytes, first_bytes);
            check!(eq; counts.key_visits, 1);
        }
        PhysicalObservationPage::Exhausted { .. } => {
            return Err("expected byte continuation".into())
        }
    }
    let cursor = continuation(first)?;
    let (last, rows) = page(&snapshot, budget(3, 4096), Some(&cursor))?;
    check!(eq; rows.iter().map(|row| row.0.value).collect::<Vec<_>>(), vec![2, 3]);
    check!(
        matches!(last, PhysicalObservationPage::Exhausted { counts } if counts.decoded_rows == 2)
    );
    Ok(())
}

#[test]
fn empty_table_accepts_zero_budgets_but_nonempty_zero_rows_refuse() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let snapshot = store.snapshot();
    let (empty, rows) = page(&snapshot, budget(0, 0), None)?;
    check!(
        matches!(empty, PhysicalObservationPage::Exhausted { counts } if counts == crate::ReadCounts::default())
    );
    check!(rows.is_empty());
    seed(&store)?;
    let current = store.snapshot();
    let result = current.for_each_physical_observation_page::<Row>(
        Table::Schools,
        budget(0, 0),
        None,
        |_, _, _, _| Err(StoreError::CounterOverflow),
    );
    check!(matches!(
        result,
        Err(StoreError::BudgetExceeded {
            what: "capture physical rows",
            needed: 1,
            available: 0
        })
    ));
    Ok(())
}

#[test]
fn cursor_rejects_foreign_store_table_and_distinct_snapshot_at_same_sequence() -> TestResult {
    let directory = tempfile::tempdir()?;
    let foreign_directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let foreign = Store::open(foreign_directory.path())?;
    seed(&store)?;
    seed(&foreign)?;
    let snapshot = store.snapshot();
    let same_sequence = store.snapshot();
    check!(eq; snapshot.sequence(), same_sequence.sequence());
    let other = foreign.snapshot();
    let (first, _) = page(&snapshot, budget(1, 1024), None)?;
    let cursor = continuation(first)?;
    for (target, table) in [
        (&snapshot, Table::Teams),
        (&same_sequence, Table::Schools),
        (&other, Table::Schools),
    ] {
        let mut calls = 0;
        let result = target.for_each_physical_observation_page::<Row>(
            table,
            budget(3, 4096),
            Some(&cursor),
            |_, _, _, _| {
                calls += 1;
                Ok(())
            },
        );
        check!(matches!(result, Err(StoreError::Refused { .. })));
        check!(eq; calls, 0);
    }
    store.entities.insert(
        crate::keys::observation_key(Table::Schools, "later", 3),
        RAW[0],
    )?;
    let newer = store.snapshot();
    let result = newer.for_each_physical_observation_page::<Row>(
        Table::Schools,
        budget(3, 4096),
        Some(&cursor),
        |_, _, _, _| Err(StoreError::CounterOverflow),
    );
    check!(matches!(result, Err(StoreError::Refused { .. })));
    Ok(())
}

#[test]
fn oversized_row_and_key_plus_value_refuse_before_decode_or_callback() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let key = crate::keys::observation_key(Table::Schools, "alpha", 0);
    store.entities.insert(&key, b"{")?;
    let snapshot = store.snapshot();
    for limits in [
        ReadBudget {
            record_bytes: 0,
            ..budget(1, 4096)
        },
        budget(1, 1),
    ] {
        let mut calls = 0;
        let result = snapshot.for_each_physical_observation_page::<Row>(
            Table::Schools,
            limits,
            None,
            |_, _, _, _| {
                calls += 1;
                Ok(())
            },
        );
        check!(matches!(
            result,
            Err(StoreError::RowTooLarge { .. }
                | StoreError::BudgetExceeded {
                    what: "capture encoded bytes",
                    ..
                })
        ));
        check!(eq; calls, 0);
    }
    Ok(())
}

#[test]
fn oversized_later_row_refuses_instead_of_returning_endless_continuation() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    store.entities.insert(
        crate::keys::observation_key(Table::Schools, "alpha", 0),
        RAW[1],
    )?;
    store.entities.insert(
        crate::keys::observation_key(Table::Schools, "beta", 1),
        vec![b'!'; 100],
    )?;
    let snapshot = store.snapshot();
    let mut calls = 0;
    let result = snapshot.for_each_physical_observation_page::<Row>(
        Table::Schools,
        budget(3, 80),
        None,
        |_, _, _, _| {
            calls += 1;
            Ok(())
        },
    );
    check!(matches!(
        result,
        Err(StoreError::BudgetExceeded {
            what: "capture encoded bytes",
            ..
        })
    ));
    check!(eq; calls, 1);
    Ok(())
}

#[test]
fn held_page_cursor_keeps_original_snapshot_after_live_changes() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    seed(&store)?;
    let snapshot = store.snapshot();
    let (first, _) = page(&snapshot, budget(1, 1024), None)?;
    let cursor = continuation(first)?;
    store.entities.insert(
        crate::keys::observation_key(Table::Schools, "alpha", 1),
        b"{",
    )?;
    store.entities.insert(
        crate::keys::observation_key(Table::Schools, "zeta", 3),
        b"{",
    )?;
    let (last, rows) = page(&snapshot, budget(3, 4096), Some(&cursor))?;
    check!(eq; rows.iter().map(|row| row.2.as_slice()).collect::<Vec<_>>(), vec![RAW[1], RAW[2]]);
    check!(matches!(last, PhysicalObservationPage::Exhausted { .. }));
    Ok(())
}

#[test]
fn page_boundary_defers_decode_and_callback_failure_remains_exact() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    seed(&store)?;
    store.entities.insert(
        crate::keys::observation_key(Table::Schools, "alpha", 1),
        b"{",
    )?;
    let snapshot = store.snapshot();
    let (first, _) = page(&snapshot, budget(1, 1024), None)?;
    let cursor = continuation(first)?;
    let result = snapshot.for_each_physical_observation_page::<Row>(
        Table::Schools,
        budget(1, 1024),
        Some(&cursor),
        |_, _, _, _| Err(StoreError::CounterOverflow),
    );
    check!(matches!(result, Err(StoreError::Decode { .. })));
    let result = snapshot.for_each_physical_observation_page::<Row>(
        Table::Schools,
        budget(3, 4096),
        None,
        |_, _, _, _| {
            Err(StoreError::Refused {
                detail: "sink refused".into(),
            })
        },
    );
    check!(matches!(result, Err(StoreError::Refused { detail }) if detail == "sink refused"));
    Ok(())
}

#[test]
fn resumed_derived_page_stops_before_other_table_or_generation() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let generation = store.snapshot().derived_generation();
    for (id, raw) in [("alpha", RAW[0]), ("beta", RAW[2])] {
        store.entities.insert(
            crate::keys::derived_key(Table::ReviewCases, generation, id.as_bytes()),
            raw,
        )?;
    }
    store.entities.insert(
        crate::keys::derived_key(
            Table::ReviewCases,
            generation.checked_add(1).ok_or("generation")?,
            b"alpha",
        ),
        b"{",
    )?;
    let snapshot = store.snapshot();
    let first = snapshot.for_each_physical_observation_page::<Row>(
        Table::ReviewCases,
        budget(1, 1024),
        None,
        |_, _, _, _| Ok(()),
    )?;
    let cursor = continuation(first)?;
    let mut ids = Vec::new();
    let last = snapshot.for_each_physical_observation_page::<Row>(
        Table::ReviewCases,
        budget(1, 1024),
        Some(&cursor),
        |row, _, _, _| {
            ids.push(row.id);
            Ok(())
        },
    )?;
    check!(eq; ids, vec!["beta"]);
    check!(
        matches!(last, PhysicalObservationPage::Exhausted { counts } if counts.decoded_rows == 1)
    );
    Ok(())
}
