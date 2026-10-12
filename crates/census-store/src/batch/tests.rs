use std::cell::Cell;

use crate::keys::observation_key;
use crate::{Store, StoreError, StoreResult, Table};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Work {
    point_reads: u64,
    key_visits: u64,
    comparisons: u64,
}

thread_local! {
    static WORK: Cell<Work> = Cell::new(Work::default());
}

pub(super) fn point_read() -> StoreResult<()> {
    WORK.with(|work| {
        let mut next = work.get();
        next.point_reads = next
            .point_reads
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        work.set(next);
        Ok(())
    })
}

pub(super) fn key_visit() -> StoreResult<()> {
    WORK.with(|work| {
        let mut next = work.get();
        next.key_visits = next
            .key_visits
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        work.set(next);
        Ok(())
    })
}

pub(super) fn comparison() -> StoreResult<()> {
    WORK.with(|work| {
        let mut next = work.get();
        next.comparisons = next
            .comparisons
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        work.set(next);
        Ok(())
    })
}

fn seed(store: &Store, ids: impl IntoIterator<Item = String>) -> StoreResult<()> {
    let mut batch = store.db.batch();
    for id in ids {
        let value = serde_json::to_vec(&json!({"id": id, "note": "old"})).map_err(|source| {
            StoreError::Json {
                detail: "seeding map history".into(),
                source,
            }
        })?;
        for sequence in [0, 7] {
            batch.insert(
                &store.entities,
                observation_key(Table::IdentityVerdicts, &id, sequence),
                &value,
            );
        }
    }
    batch
        .commit()
        .map_err(|source| StoreError::Write { source })
}

fn held(
    store: &Store,
    id: &str,
    sequence: u64,
) -> Result<Option<Value>, Box<dyn std::error::Error>> {
    store
        .entities
        .get(observation_key(Table::IdentityVerdicts, id, sequence))?
        .map(|value| serde_json::from_slice(&value).map_err(Into::into))
        .transpose()
}

#[test]
fn derived_map_update_visits_only_named_ids() -> TestResult {
    let mut measurements = Vec::new();
    for physical_ids in [4_u64, 128, 4096] {
        for updated in [1_u64, 4] {
            let dir = tempfile::tempdir()?;
            let store = Store::open(dir.path())?;
            seed(&store, (0..physical_ids).map(|id| format!("row-{id:05}")))?;
            let records: Vec<_> = (0..updated)
                .map(|id| json!({"id": format!("row-{id:05}"), "note": "new"}))
                .collect();
            WORK.with(|work| work.set(Work::default()));
            if updated == 1 {
                store.replace_many(Table::IdentityVerdicts, &records)?;
            } else {
                let mut batch = store.write_batch();
                batch.replace_many(Table::IdentityVerdicts, &records)?;
                batch.commit()?;
            }
            let work = WORK.with(Cell::get);
            println!(
                "physical_keys={} updated={updated} work={work:?}",
                physical_ids.checked_mul(2).ok_or("physical key count")?
            );
            measurements.push((updated, work));
            for id in 0..updated {
                let id = format!("row-{id:05}");
                check!(eq; held(&store, &id, 0)?, Some(json!({"id": id, "note": "new"})));
                check!(eq; held(&store, &id, 7)?, None);
            }
            if physical_ids > updated {
                let id = format!("row-{updated:05}");
                check!(eq; held(&store, &id, 7)?, Some(json!({"id": id, "note": "old"})));
            }
        }
    }
    for (updated, work) in measurements {
        check!(eq; work, Work { point_reads: updated, key_visits: updated.checked_mul(2).ok_or("visit bound")?, comparisons: updated });
    }
    Ok(())
}

#[test]
fn derived_map_cleanup_preserves_nul_suffixed_neighbours() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    seed(&store, ["x", "x\0neighbor", "xy", "z"].map(str::to_string))?;
    WORK.with(|work| work.set(Work::default()));
    store.replace(Table::IdentityVerdicts, &json!({"id": "x", "note": "new"}))?;
    let work = WORK.with(Cell::get);
    println!("nul_neighbours work={work:?}");
    check!(eq; held(&store, "x", 0)?, Some(json!({"id": "x", "note": "new"})));
    check!(eq; held(&store, "x", 7)?, None);
    for id in ["x\0neighbor", "xy", "z"] {
        for sequence in [0, 7] {
            check!(eq; held(&store, id, sequence)?, Some(json!({"id": id, "note": "old"})));
        }
    }
    check!(eq; work, Work { point_reads: 1, key_visits: 4, comparisons: 2 });
    Ok(())
}

#[test]
fn overlapping_named_prefixes_visit_each_physical_key_once() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let ids = ["x", "x\0neighbor", "x\0neighbor\0child"];
    seed(&store, ids.map(str::to_string))?;
    let records: Vec<_> = ids.map(|id| json!({"id": id, "note": "new"})).into();
    WORK.with(|work| work.set(Work::default()));
    store.replace_many(Table::IdentityVerdicts, &records)?;
    check!(eq; WORK.with(Cell::get), Work { point_reads: 3, key_visits: 6, comparisons: 3 });
    for id in ids {
        check!(eq; held(&store, id, 0)?, Some(json!({"id": id, "note": "new"})));
        check!(eq; held(&store, id, 7)?, None);
    }
    Ok(())
}

#[test]
fn cleanup_budget_admits_the_table_ceiling_and_refuses_the_next_visit() -> TestResult {
    let mut visits = crate::MAX_ROWS_PER_TABLE
        .checked_sub(1)
        .ok_or("table ceiling")?;
    super::charge_cleanup(&mut visits)?;
    check!(eq; visits, crate::MAX_ROWS_PER_TABLE);
    check!(matches!(
        super::charge_cleanup(&mut visits),
        Err(StoreError::BudgetExceeded {
            what: "derived map cleanup key visits",
            needed: 20_000_001,
            available: 20_000_000,
        })
    ));
    check!(eq; visits, crate::MAX_ROWS_PER_TABLE);
    visits = u64::MAX;
    check!(matches!(
        super::charge_cleanup(&mut visits),
        Err(StoreError::CounterOverflow)
    ));
    check!(eq; visits, u64::MAX);
    Ok(())
}

#[test]
fn sequence_zero_maps_scale_with_the_named_rows_and_empty_updates_do_no_work() -> TestResult {
    for table in [
        Table::SourceAccess,
        Table::IdentityVerdicts,
        Table::AthleteIdentityDecisions,
        Table::CoachLanes,
    ] {
        let dir = tempfile::tempdir()?;
        let store = Store::open(dir.path())?;
        let rows: Vec<_> = (0..4096)
            .map(|id| json!({"id": format!("row-{id:05}"), "note": "old"}))
            .collect();
        store.replace_many(table, &rows)?;
        WORK.with(|work| work.set(Work::default()));
        store.replace_many::<Value>(table, &[])?;
        check!(eq; WORK.with(Cell::get), Work::default());
        store.replace_many(
            table,
            &[
                json!({"id": "row-00000", "note": "first"}),
                json!({"id": "row-00000", "note": "last"}),
            ],
        )?;
        check!(eq; WORK.with(Cell::get), Work { point_reads: 2, key_visits: 1, comparisons: 0 });
        let held: Value = serde_json::from_slice(
            &store
                .entities
                .get(observation_key(table, "row-00000", 0))?
                .ok_or("updated row")?,
        )?;
        check!(eq; held, json!({"id": "row-00000", "note": "last"}));
        check!(eq; store.count(table)?, 4096);
    }
    Ok(())
}
