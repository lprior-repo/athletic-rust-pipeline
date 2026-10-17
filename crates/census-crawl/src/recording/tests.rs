use super::*;
use census_store::{Store, Table};
use serde::Serialize;
use serde_json::json;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn scratch() -> TestResult<(tempfile::TempDir, Store)> {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    Ok((dir, store))
}

#[test]
fn the_store_route_writes_on_commit() -> TestResult {
    let (_dir, store) = scratch()?;
    let mut dropped = RowSink::Store(&store).write_batch();
    dropped.append_many(Table::Meets, &[json!({"id": "m1"})])?;
    drop(dropped);
    check!(eq; store.walk_table(Table::Meets)?.rows, 0);

    let mut batch = RowSink::Store(&store).write_batch();
    batch.append_many(Table::Meets, &[json!({"id": "m1"}), json!({"id": "m2"})])?;
    batch.commit()?;
    check!(eq; store.walk_table(Table::Meets)?.rows, 2);
    Ok(())
}

#[test]
fn the_recording_route_holds_rows_and_leaves_the_store_alone() -> TestResult {
    let (_dir, store) = scratch()?;
    let recording = Recording::new();
    let mut batch = RowSink::Record {
        store: &store,
        recording: &recording,
    }
    .write_batch();
    batch.append_many(Table::Meets, &[json!({"id": "m1"})])?;
    batch.append(Table::Performances, vec![json!({"id": "p1"})])?;
    batch.journal_done("milesplit_results_v1", "set-1", &json!({"url": "u"}))?;
    check!(
        recording.is_empty(),
        "nothing is recorded before the commit"
    );
    batch.commit()?;

    check!(eq; store.walk_table(Table::Meets)?.rows, 0);
    check!(eq; store.walk_table(Table::Performances)?.rows, 0);
    check!(eq; recording.rows(), 2);
    let drained = recording.drain();
    check!(eq; drained.rows.len(), 2);
    check!(eq; drained.rows[0].table, Table::Meets);
    check!(eq; drained.rows[0].rows, vec![json!({"id": "m1"})]);
    check!(eq; drained.rows[1].table, Table::Performances);
    check!(eq;
        drained.journal,
        vec![RecordedJournal {
            phase: "milesplit_results_v1".to_string(),
            key: "set-1".to_string(),
            payload: json!({"url": "u"}),
        }],
        "the unit marker rides the recording, not the store"
    );
    check!(recording.is_empty(), "a drain takes what it returns");
    Ok(())
}

#[test]
fn a_recorded_unit_is_not_journaled_until_it_is_written() -> TestResult {
    let (_dir, store) = scratch()?;
    let recording = Recording::new();
    let mut batch = RowSink::Record {
        store: &store,
        recording: &recording,
    }
    .write_batch();
    batch.journal_done(
        "wayzata_schedule_v1",
        "page-1",
        &json!({"season": "2026-27"}),
    )?;
    batch.commit()?;
    check!(eq;
        store
            .journal_keys("wayzata_schedule_v1")
            ?
            .len(),
        0,
        "a marker the store cannot see is a marker no reader acts on"
    );
    check!(eq; recording.drain().journal.len(), 1);
    Ok(())
}

#[test]
fn bounded_rows_keep_individual_node_limits_and_exact_batch_conservation() -> TestResult {
    #[derive(Serialize)]
    struct Row {
        id: u16,
        values: Vec<u8>,
    }
    let (_dir, store) = scratch()?;
    let recording = Recording::new();
    let rows: Vec<_> = (0..1000)
        .map(|id| Row {
            id,
            values: vec![7; 100],
        })
        .collect();
    let sink = RowSink::Record {
        store: &store,
        recording: &recording,
    };
    let mut batch = sink.write_batch();
    batch.append_many(Table::Meets, &rows)?;
    batch.commit()?;
    check!(eq; recording.rows(), rows.len());
    let oversized = Row {
        id: 1000,
        values: vec![7; MAX_RECORDED_WORK],
    };
    let mut refused = sink.write_batch();
    check!(matches!(
        refused.append_many(Table::Meets, &[oversized]),
        Err(crate::CrawlError::Resource {
            resource: "recording serialization work",
            ..
        })
    ));
    drop(refused);
    check!(eq; recording.rows(), rows.len());
    check!(eq; store.walk_table(Table::Meets)?.rows, 0);
    let drained = recording.drain();
    check!(eq; drained.rows.len(), 1);
    let retained = drained.rows.first().ok_or("recorded batch")?;
    check!(eq; retained.table, Table::Meets);
    check!(eq; retained.rows.len(), rows.len());
    for (retained, original) in retained.rows.iter().zip(rows) {
        check!(eq; retained, &serde_json::to_value(original)?);
    }
    Ok(())
}
