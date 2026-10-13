use super::*;
use fjall::{PersistMode, Readable};

const CHILD_ROOT: &str = "CENSUS_STORE_READBACK_CHILD_ROOT";
const CHILD_SCAN: &str = "CENSUS_STORE_READBACK_CHILD_SCAN";
const INTERRUPTED: i32 = 23;

fn durable_write(store: &Store, rows: &[(Vec<u8>, Vec<u8>)]) -> TestResult {
    let mut batch = store.db.batch();
    for (key, value) in rows {
        batch.insert(&store.journal, key, value);
    }
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    Ok(())
}

fn interrupt_writer(root: &std::path::Path, scan: bool) -> TestResult {
    let store = Store::open(root)?;
    store.append(
        crate::Table::Schools,
        &crate::tests::school("Durable Readback"),
    )?;
    let first = plan(&[large("first"), record("second", json!({"rows": 2}))])?;
    let mut committed = pairs(&first, &first.id)?;
    committed.push(alias_pair(&first.id)?);
    durable_write(&store, &committed)?;
    if scan {
        let rows = store.scan::<census_domain::model::CanonicalSchool>(crate::Table::Schools)?;
        check!(eq; rows.len(), 1);
        check!(eq;
            rows.first().map(|row| row.name.as_str()),
            Some("Durable Readback")
        );
    }
    let second = plan(&[large("replacement"), record("pending", json!({"rows": 9}))])?;
    let pending = pairs(&second, &second.id)?;
    let first_chunk = pending.first().ok_or("replacement has no chunk")?;
    durable_write(&store, std::slice::from_ref(first_chunk))?;
    let mut abandoned = store.db.batch();
    for (key, value) in pending {
        abandoned.insert(&store.journal, key, value);
    }
    let (key, value) = alias_pair(&second.id)?;
    abandoned.insert(&store.journal, key, value);
    std::process::exit(INTERRUPTED);
}

fn readback(root: &std::path::Path, scan: bool) -> TestResult {
    let first = plan(&[large("first"), record("second", json!({"rows": 2}))])?;
    let second = plan(&[large("replacement"), record("pending", json!({"rows": 9}))])?;
    let store = Store::open(root)?;
    let snapshot = store.db.snapshot();
    check!(snapshot
        .get(&store.journal, object_manifest_key(&first.id)?)?
        .is_some());
    check!(snapshot
        .get(&store.journal, object_index_key(&first.id, 0)?)?
        .is_some());
    for index in 0..first.chunks.len() {
        check!(snapshot
            .get(&store.journal, object_chunk_key(&first.id, index)?)?
            .is_some());
    }
    check!(snapshot
        .get(&store.journal, object_chunk_key(&second.id, 0)?)?
        .is_some());
    check!(snapshot
        .get(&store.journal, object_manifest_key(&second.id)?)?
        .is_none());
    let object = store
        .journal_object(PHASE, KEY)?
        .ok_or("ready generation missing")?;
    check!(eq; object.object_id(), first.id);
    check!(eq; object.chunk_count(), first.chunks.len());
    check!(eq; object.total_bytes(), first.payload.len());
    let records = stream(&object)?;
    check!(eq; records.len(), 2);
    check!(eq;
        records.first().map(|row| row.entry().key.as_str()),
        Some("first")
    );
    check!(eq;
        records.first().map(JournalRecord::payload),
        Some(&json!({"blob": "x".repeat(600 * 1024)}))
    );
    check!(eq;
        records.get(1).map(JournalRecord::payload),
        Some(&json!({"rows": 2}))
    );
    let rows = store.scan::<census_domain::model::CanonicalSchool>(crate::Table::Schools)?;
    check!(eq; rows.len(), 1);
    check!(eq;
        rows.first().map(|row| row.name.as_str()),
        Some("Durable Readback")
    );
    println!(
        "readback scan_before_interrupt={scan} exit={INTERRUPTED} object={} manifest=1 index_pages=1 chunks={} bytes={} records={} schools={} partial_replacement_chunks=1 replacement_manifest=absent",
        object.object_id(), object.chunk_count(), object.total_bytes(), records.len(), rows.len()
    );
    Ok(())
}

#[test]
fn interrupted_journal_write_reopens_the_complete_generation_with_or_without_scan() -> TestResult {
    if let Some(root) = std::env::var_os(CHILD_ROOT) {
        return interrupt_writer(
            std::path::Path::new(&root),
            std::env::var_os(CHILD_SCAN).is_some(),
        );
    }
    let (_, module) = module_path!()
        .split_once("::")
        .ok_or("test module has no crate")?;
    let test = format!(
        "{}::interrupted_journal_write_reopens_the_complete_generation_with_or_without_scan",
        module
    );
    for scan in [false, true] {
        let root = tempfile::tempdir()?;
        let mut command = std::process::Command::new(std::env::current_exe()?);
        command.args(["--exact", &test, "--nocapture"]);
        command.env(CHILD_ROOT, root.path());
        command.env_remove(CHILD_SCAN);
        if scan {
            command.env(CHILD_SCAN, "1");
        }
        let output = command.output()?;
        check!(eq;
            output.status.code(),
            Some(INTERRUPTED),
            "child stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        readback(root.path(), scan)?;
    }
    Ok(())
}

#[test]
fn a_read_manifest_and_index_keep_their_chunks_when_live_rows_are_overwritten() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let first = stage_records(&store, &entries(&[("alpha", 1), ("beta", 2)]))?;
    let snapshot = store.db.snapshot();
    let raw_manifest = snapshot
        .get(&store.journal, object_manifest_key(&first.id)?)?
        .ok_or("manifest missing before overwrite")?;
    check!(eq;
        serde_json::from_slice::<Value>(&raw_manifest)?,
        manifest_record(&first.id, &first)
    );
    let raw_index = snapshot
        .get(&store.journal, object_index_key(&first.id, 0)?)?
        .ok_or("index missing before overwrite")?;
    check!(eq;
        serde_json::from_slice::<Value>(&raw_index)?,
        index_record(&first.id, 0, &first.ranges, 0)
    );
    let second = plan(&entries(&[("alpha", 9), ("beta", 8)]))?;
    check!(ne; first.id, second.id);
    stage_under(&store, &second, &first.id)?;
    let captured = crate::read::StoreSnapshot::new(
        snapshot,
        &store,
        store.derived_generation(),
        store.evidence_generation(),
    );
    let object = captured
        .journal_object(PHASE, KEY)?
        .ok_or("pinned generation missing")?;
    check!(eq; object.object_id(), first.id);
    let records = stream(&object)?;
    check!(eq; records.len(), 2);
    check!(eq;
        records.first().map(JournalRecord::payload),
        Some(&json!({"rows": 1}))
    );
    check!(eq;
        records.get(1).map(JournalRecord::payload),
        Some(&json!({"rows": 2}))
    );
    check!(matches!(
        store.journal_object(PHASE, KEY),
        Err(StoreError::JournalObjectDigest { declared, computed, .. })
            if declared == first.id && computed == second.id
    ));
    Ok(())
}
