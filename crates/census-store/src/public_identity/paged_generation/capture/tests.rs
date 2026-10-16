use super::super::{AuthorityTableV2, MAX_RAW_VALUE_BYTES};
use super::*;
use crate::public_identity_tests::support::{fixture, subject, TestResult};
use crate::{Entity, Table};
use fjall::PersistMode;

mod concurrency;
mod fences;
mod layouts;
#[cfg(unix)]
mod owned_child;
#[cfg(unix)]
mod process_crash;
mod recovery;
mod refusal;

fn enrolled() -> TestResult<(tempfile::TempDir, Store, PublicIdentityWriterV1)> {
    let (directory, store, writer) = fixture()?;
    writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
    Ok((directory, store, writer))
}

fn seeded_and_reopened(
    count: u64,
) -> TestResult<(tempfile::TempDir, Store, PublicIdentityWriterV1)> {
    let (directory, store, writer) = crate::public_identity_tests::support::unenrolled_fixture()?;
    seed(&store, count)?;
    let root = store.root().to_path_buf();
    drop(writer);
    drop(store);
    let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
    Ok((directory, store, writer))
}

fn operation(label: &str) -> StoreResult<RawGenerationOperationIdV2> {
    RawGenerationOperationIdV2::new(label)
}

fn raw(index: u64) -> TestResult<(Vec<u8>, Vec<u8>)> {
    let athlete = subject("raw capture é", "4711")?;
    let key = crate::keys::observation_key(Table::Athletes, athlete.entity_id(), index);
    let compact = serde_json::to_string(&athlete)?;
    let text = match index % 3 {
        0 => format!(" \n{}\t", serde_json::to_string_pretty(&athlete)?),
        1 => compact.replace("4711", "\\u0034711"),
        _ => compact,
    };
    Ok((key, text.into_bytes()))
}

fn seed(store: &Store, count: u64) -> TestResult {
    let mut batch = store.db.batch();
    for index in 0..count {
        let (key, value) = raw(index)?;
        batch.insert(&store.entities, key, value);
    }
    marks(store, &mut batch, count);
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    Ok(())
}

fn marks(store: &Store, batch: &mut fjall::OwnedWriteBatch, count: u64) {
    for prefix in ["sequence", "rows"] {
        crate::meta::put_text(
            batch,
            &store.meta,
            &format!("{prefix}:{}", Table::Athletes.file()),
            &count.to_string(),
        );
    }
}

fn retained_rows(store: &Store, retained: &RetainedRawGenerationV2) -> TestResult<u64> {
    let snapshot = store.snapshot();
    let seal = retained.verify(&snapshot)?;
    let binding = retained
        .binding()
        .table_binding(AuthorityTableV2::Athletes)?;
    let reference = seal.tables().first().ok_or("athlete table")?;
    let table = snapshot.raw_generation_table(&reference.object(), &binding)?;
    let mut index = 0_u64;
    for block in table.blocks() {
        let block = snapshot.raw_generation_block(&block.object(), &binding)?;
        for page in block.pages() {
            let page = snapshot.raw_generation_page(&page.object(), &binding)?;
            for row in page.rows() {
                let (key, value) = raw(index)?;
                assert_eq!(row.physical_key(), key);
                assert_eq!(row.raw_value(), value);
                index = index.checked_add(1).ok_or("row count")?;
            }
        }
    }
    Ok(index)
}

#[test]
fn captures_more_than_8192_raw_variants_and_five_empty_tables() -> TestResult {
    let (_directory, store, writer) = enrolled()?;
    seed(&store, 8_193)?;
    let outcome = writer.capture_raw_generation_v2(&store, &operation("large-raw")?)?;
    assert!(matches!(
        &outcome,
        RawGenerationCaptureOutcomeV2::Captured(_)
    ));
    let retained = outcome.retained().clone();
    assert_eq!(retained_rows(&store, &retained)?, 8_193);
    let seal = retained.verify(&store.snapshot())?;
    assert_eq!(seal.declared_inventory().pages(), 17);
    assert_eq!(seal.declared_inventory().rows(), 8_193);
    for table in seal.tables().iter().skip(1) {
        assert_eq!(table.inventory().rows(), 0);
    }
    Ok(())
}

#[test]
fn completely_empty_capture_retains_six_exhausted_tables() -> TestResult {
    let (_directory, store, writer) = enrolled()?;
    let retained = writer
        .capture_raw_generation_v2(&store, &operation("empty")?)?
        .retained()
        .clone();
    let seal = retained.verify(&store.snapshot())?;
    assert_eq!(seal.declared_inventory().rows(), 0);
    assert_eq!(seal.declared_inventory().object_count(), 6);
    assert_eq!(
        seal.inventory_including_seal(retained.seal())?
            .object_count(),
        7
    );
    Ok(())
}
