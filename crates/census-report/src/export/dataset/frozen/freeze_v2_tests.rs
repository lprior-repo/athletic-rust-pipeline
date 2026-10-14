use super::fixtures::{athlete, school, seeded_store, TestResult};
use super::objects;
use super::reader::ChunkReader;
use super::stream::StreamKind;
use super::verify;
use super::{freeze_v2, to_export_dataset};
use crate::export::ExportDataset;
use census_domain::model::CanonicalAthlete;
use census_store::{InputState, Store, Table};

const HUGE_ROW_BYTES: usize = 200 * 1024;
const PAYLOAD_FLOOR: u64 = 4_915_200;
const PAYLOAD_CEILING: u64 = 8_388_608;

#[test]
fn freeze_v2_installs_and_reuses_a_coherent_root() -> TestResult {
    let seeded = seeded_store()?;
    let store = seeded.store;
    store.append(Table::Schools, &school("Reuse High"))?;
    let first = freeze_v2(&store, "freeze-v2:reuse")?;
    let second = freeze_v2(&store, "freeze-v2:reuse")?;
    check!(eq; second.manifest_digest(), first.manifest_digest());
    check!(eq; second.attempt(), first.attempt());
    check!(eq; second.bytes(), first.bytes());
    let record = store
        .get_input_record(first.attempt())?
        .ok_or("a frozen root retains its durable input record")?;
    check!(eq; record.state(), InputState::Frozen);
    check!(eq; record.bytes(), first.bytes());
    check!(
        eq;
        record.chunks().first().map(String::as_str),
        Some(first.manifest_digest())
    );
    let epoch = store.lease_acquire(first.attempt(), "freeze-v2-probe", "2999-12-31")?;
    store.lease_release(first.attempt(), epoch)?;
    drop(store);
    let reopened = Store::open(seeded.directory.path())?;
    let frozen = to_export_dataset(&first, &reopened)?;
    let live = ExportDataset::load(&reopened)?;
    check!(eq; frozen.lineage.input_digest, live.lineage.input_digest);
    check!(eq; frozen.lineage.source_digest, live.lineage.source_digest);
    check!(eq; frozen.athletes.len(), live.athletes.len());
    check!(eq; frozen.schools.len(), live.schools.len());
    check!(eq; frozen.teams.len(), live.teams.len());
    Ok(())
}

#[test]
fn freeze_v2_streams_bounded_chunks_in_encounter_order() -> TestResult {
    let seeded = seeded_store()?;
    let store = seeded.store;
    let anchor = school("Bounded High");
    store.append(Table::Schools, &anchor)?;
    let mut appended: u64 = 0;
    for index in 0..24_u32 {
        let mut row = athlete(
            &anchor.id,
            &format!("Huge Runner {index}"),
            &format!("huge-{index}"),
        )?;
        row.canonical_name = "x".repeat(HUGE_ROW_BYTES);
        store.append(Table::Athletes, &row)?;
        appended = appended.checked_add(1).ok_or("row count overflow")?;
    }
    let root = freeze_v2(&store, "freeze-v2:bounded")?;
    let objects_dir = objects::directory(root.directory());
    let reference = root
        .manifest()
        .stream(StreamKind::Athletes)
        .ok_or("the frozen root carries the athletes stream")?;
    let page = verify::page(&objects_dir, reference)?;
    check!(
        page.chunks().len() >= 2,
        "a bounded stream seals several chunks"
    );
    let mut total: u64 = 0;
    for descriptor in page.chunks() {
        check!(descriptor.bytes <= objects::CHUNK_CEILING);
        total = total
            .checked_add(descriptor.bytes)
            .ok_or("payload overflow")?;
    }
    check!(
        total >= PAYLOAD_FLOOR,
        "the grounded payload reaches the objects"
    );
    check!(total <= PAYLOAD_CEILING, "the frozen payload stays bounded");
    check!(eq; page.bytes(), total);
    let mut ids: Vec<String> = Vec::new();
    for descriptor in page.chunks() {
        let bytes = objects::read(&objects_dir, &descriptor.sha256, objects::CHUNK_CEILING)?;
        let mut reader = ChunkReader::open(StreamKind::Athletes, &bytes, descriptor.first_ordinal)?;
        while let Some(decoded) = reader.next_record::<CanonicalAthlete>()? {
            ids.push(decoded.entity_id);
        }
    }
    let expected = appended.checked_add(2).ok_or("row count overflow")?;
    check!(eq; u64::try_from(ids.len())?, expected);
    let mut sorted = ids.clone();
    sorted.sort();
    check!(eq; ids, sorted);
    check!(eq; page.rows(), expected);
    Ok(())
}

#[test]
fn freeze_v2_refuses_to_replace_a_differing_object() -> TestResult {
    let seeded = seeded_store()?;
    let store = seeded.store;
    let root = freeze_v2(&store, "freeze-v2:no-replace")?;
    let objects_dir = objects::directory(root.directory());
    let reference = root
        .manifest()
        .stream(StreamKind::Athletes)
        .ok_or("the frozen root carries the athletes stream")?;
    let page = verify::page(&objects_dir, reference)?;
    let victim = page
        .chunks()
        .first()
        .ok_or("the athletes stream holds a chunk")?;
    let path = objects_dir.join(&victim.sha256);
    let original = std::fs::read(&path)?;
    check!(eq; objects::install(&objects_dir, &original)?, victim.sha256);
    std::fs::remove_file(&path)?;
    std::fs::write(&path, b"different bytes")?;
    let error = match objects::install(&objects_dir, &original) {
        Ok(_) => return Err("a differing object was replaced".into()),
        Err(error) => error,
    };
    check!(error.to_string().contains("refusing to replace"));
    check!(eq; std::fs::read(&path)?, b"different bytes".to_vec());
    Ok(())
}

#[test]
fn freeze_v2_refuses_a_concurrent_capture() -> TestResult {
    let seeded = seeded_store()?;
    let store = seeded.store;
    let directory = super::freeze::directory(&store, "freeze-v2:locked")?;
    std::fs::create_dir_all(&directory)?;
    let held = super::capture_lock(&directory.join("lock"))?;
    let error = match freeze_v2(&store, "freeze-v2:locked") {
        Ok(_) => return Err("a locked capture was accepted".into()),
        Err(error) => error,
    };
    check!(error.to_string().contains("already being captured"));
    drop(held);
    let root = freeze_v2(&store, "freeze-v2:locked")?;
    check!(eq; root.manifest().streams.len(), StreamKind::ALL.len());
    Ok(())
}
