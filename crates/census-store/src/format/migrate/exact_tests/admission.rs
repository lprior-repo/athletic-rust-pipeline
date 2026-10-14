use super::*;

const CACHE_BYTES: u64 = 8 * 1024 * 1024;
const PERFORMANCE_ID: &str = "perf_admission_synthetic";

#[derive(Debug, PartialEq, Eq)]
struct RetainedBytes {
    entities: BTreeMap<Vec<u8>, Vec<u8>>,
    journal: BTreeMap<Vec<u8>, Vec<u8>>,
    meta: BTreeMap<Vec<u8>, Vec<u8>>,
    receipts: BTreeMap<Vec<u8>, Vec<u8>>,
}

fn retained_bytes(root: &Path) -> TestResult<RetainedBytes> {
    let (_, entities, journal, meta, receipts) = crate::open_keyspaces(root, CACHE_BYTES)?;
    Ok(RetainedBytes {
        entities: all_bytes(&entities)?,
        journal: all_bytes(&journal)?,
        meta: all_bytes(&meta)?,
        receipts: all_bytes(&receipts)?,
    })
}

fn seed_retained(root: &Path, time: Value) -> TestResult<Store> {
    let store = Store::open_with(root, CACHE_BYTES)?;
    store.append(Table::Performances, &performance(PERFORMANCE_ID, time))?;
    store.append(
        Table::SourceObservations,
        &json!({"id":"source_admission_synthetic",
            "provider":{"mark":{"TimeSeconds":1094},"capture":"retained"}}),
    )?;
    let mut effect = store.write_batch();
    effect.journal_done(
        "admission_raw_capture",
        "synthetic-capture",
        &json!({"TimeSeconds":1094,"capture":"retained"}),
    )?;
    effect.commit_once("admission-retained-effect", "admission-retained-digest")?;
    let mut batch = store.db.batch();
    meta::put_text(&mut batch, &store.meta, "admission:retained", "unchanged");
    batch.commit()?;
    Ok(store)
}

fn metadata(store: &Store, writes: &[(&str, Option<&str>)]) -> TestResult {
    let mut batch = store.db.batch();
    writes.iter().for_each(|(key, value)| match value {
        Some(value) => meta::put_text(&mut batch, &store.meta, key, value),
        None => batch.remove(&store.meta, *key),
    });
    batch.commit()?;
    Ok(())
}

fn schema(store: &Store, revision: Option<u32>) -> TestResult {
    match revision {
        None => erase_current_schema(store),
        Some(revision) => metadata(
            store,
            &[
                ("schema:store_version", Some(&revision.to_string())),
                (
                    "schema:key_format",
                    Some(if revision == 0 { "0" } else { "1" }),
                ),
            ],
        ),
    }
}

fn preserved_admission(root: &Path) -> TestResult<[Result<(), StoreError>; 2]> {
    let before = retained_bytes(root)?;
    let opened = Store::open_with(root, CACHE_BYTES).map(|_| ());
    let after_open = retained_bytes(root)?;
    let migrated = Store::migrate_with(root, CACHE_BYTES).map(|_| ());
    let after_migrate = retained_bytes(root)?;
    check!(eq; after_open, before);
    check!(eq; after_migrate, before);
    Ok([opened, migrated])
}

fn newer(root: &Path, what: &str, found: u32, supported: u32) -> TestResult {
    preserved_admission(root)?
        .into_iter()
        .try_for_each(|outcome| -> TestResult {
            match outcome {
                Err(StoreError::FormatNewer {
                    what: actual,
                    found: actual_found,
                    supported: actual_supported,
                }) => {
                    check!(eq; (actual, actual_found, actual_supported), (what, found, supported))
                }
                outcome => {
                    return Err(format!("expected newer {what} refusal, found {outcome:?}").into())
                }
            }
            Ok(())
        })
}

#[test]
fn future_lifecycle_revision_refuses_every_schema_and_supported_marker_without_writes() -> TestResult
{
    let supported = crate::format::LIFECYCLE_REVISION;
    let next = supported
        .checked_add(1)
        .ok_or("future lifecycle revision")?;
    [Some(2), Some(1), Some(0), None]
        .into_iter()
        .try_for_each(|revision| -> TestResult {
            [None, Some("1"), Some("2")]
                .into_iter()
                .try_for_each(|marker| -> TestResult {
                    [next, u32::MAX].into_iter().try_for_each(|future| {
                        let dir = tempfile::tempdir()?;
                        let store = seed_retained(dir.path(), json!(1094))?;
                        schema(&store, revision)?;
                        metadata(
                            &store,
                            &[
                                ("schema:lifecycle_revision", Some(&future.to_string())),
                                ("schema:migrating_to", marker),
                                ("rows:performances", None),
                                ("sequence:performances", None),
                            ],
                        )?;
                        drop(store);
                        newer(dir.path(), "lifecycle revision", future, supported)
                    })
                })
        })
}

#[test]
fn future_schema_beside_a_supported_marker_refuses_without_writes() -> TestResult {
    ["1", "2"].into_iter().try_for_each(|marker| {
        [3_u32, u32::MAX].into_iter().try_for_each(|future| {
            let dir = tempfile::tempdir()?;
            let store = seed_retained(dir.path(), json!(1094))?;
            metadata(
                &store,
                &[
                    ("schema:store_version", Some(&future.to_string())),
                    ("schema:migrating_to", Some(marker)),
                ],
            )?;
            drop(store);
            newer(dir.path(), "schema version", future, 2)
        })
    })
}

#[test]
fn future_key_format_beside_a_supported_marker_refuses_without_writes() -> TestResult {
    ["1", "2"].into_iter().try_for_each(|marker| {
        [2_u32, u32::MAX].into_iter().try_for_each(|future| {
            let dir = tempfile::tempdir()?;
            let store = seed_retained(dir.path(), json!(1094))?;
            metadata(
                &store,
                &[
                    ("schema:key_format", Some(&future.to_string())),
                    ("schema:migrating_to", Some(marker)),
                ],
            )?;
            drop(store);
            newer(dir.path(), "key format", future, 1)
        })
    })
}

fn current_marker_refusal(marker: &str) -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = seed_retained(dir.path(), json!(1094))?;
    metadata(&store, &[("schema:migrating_to", Some(marker))])?;
    drop(store);
    let original = raw_performance(dir.path(), PERFORMANCE_ID, 0)?;
    preserved_admission(dir.path())?
        .into_iter()
        .try_for_each(|outcome| -> TestResult {
            check!(matches!(outcome, Err(StoreError::SchemaUnknown { .. })));
            Ok(())
        })?;
    check!(eq; raw_performance(dir.path(), PERFORMANCE_ID, 0)?, original);
    let held: Value = serde_json::from_slice(&original)?;
    check!(eq; held["mark"]["TimeSeconds"], json!(1094));
    Ok(())
}

#[test]
fn schema2_marker1_is_inconsistent_and_refuses_without_writes() -> TestResult {
    current_marker_refusal("1")
}

#[test]
fn schema2_marker2_retains_integer_marks_without_legacy_reinterpretation() -> TestResult {
    current_marker_refusal("2")
}

fn migrated_entities(before: &RetainedBytes) -> TestResult<BTreeMap<Vec<u8>, Vec<u8>>> {
    let mut entities = before.entities.clone();
    entities.insert(
        observation_key(Table::Performances, PERFORMANCE_ID, 0),
        serde_json::to_vec(&performance(PERFORMANCE_ID, expected_time()))?,
    );
    Ok(entities)
}

#[test]
fn legacy_and_schema0_markers_one_and_two_resume_preserving_raw_captures_and_receipts() -> TestResult
{
    [None, Some(0)].into_iter().try_for_each(|revision| {
        ["1", "2"].into_iter().try_for_each(|marker| -> TestResult {
            let dir = tempfile::tempdir()?;
            let store = seed_retained(dir.path(), json!(1094))?;
            schema(&store, revision)?;
            metadata(
                &store,
                &[
                    ("schema:migrating_to", Some(marker)),
                    ("schema:lifecycle_revision", None),
                ],
            )?;
            drop(store);
            let before = retained_bytes(dir.path())?;
            check!(matches!(
                Store::open_with(dir.path(), CACHE_BYTES),
                Err(StoreError::MigrationRequired { .. })
            ));
            check!(eq; retained_bytes(dir.path())?, before);
            let report = Store::migrate_with(dir.path(), CACHE_BYTES)?;
            check!(eq; (report.from_schema, report.to_schema, report.to_key_format, report.rewritten_rows, report.dropped_rows, report.already_current, report.integrity_ok), (revision, 2, 1, 1, 0, false, true));
            let after = retained_bytes(dir.path())?;
            check!(eq; after.entities, migrated_entities(&before)?);
            check!(eq; after.journal, before.journal);
            check!(eq; after.receipts, before.receipts);
            let format = Store::format(dir.path())?;
            check!(eq; (format.schema_version, format.key_format, format.lifecycle_revision, format.migration_target), (Some(2), Some(1), None, None));
            let store = Store::open_with(dir.path(), CACHE_BYTES)?;
            check!(eq; store.derived_generation(), 1);
            Ok(())
        })
    })
}

#[test]
fn schema1_markers_one_and_two_resume_with_only_the_expected_rows_and_metadata_changed(
) -> TestResult {
    ["1", "2"].into_iter().try_for_each(|marker| -> TestResult {
        let dir = tempfile::tempdir()?;
        let store = seed_retained(dir.path(), json!(1094))?;
        schema(&store, Some(1))?;
        metadata(
            &store,
            &[
                ("schema:migrating_to", Some(marker)),
                ("schema:lifecycle_revision", None),
            ],
        )?;
        drop(store);
        let before = retained_bytes(dir.path())?;
        check!(matches!(
            Store::open_with(dir.path(), CACHE_BYTES),
            Err(StoreError::MigrationRequired { .. })
        ));
        check!(eq; retained_bytes(dir.path())?, before);
        let report = Store::migrate_with(dir.path(), CACHE_BYTES)?;
        check!(eq; (report.from_schema, report.to_schema, report.to_key_format, report.rewritten_rows, report.dropped_rows, report.already_current, report.integrity_ok), (Some(1), 2, 1, 1, 0, false, true));
        let mut expected_meta = before.meta.clone();
        expected_meta.insert(b"schema:store_version".to_vec(), b"2".to_vec());
        expected_meta.remove(b"schema:migrating_to".as_slice());
        let after = retained_bytes(dir.path())?;
        check!(eq; after.entities, migrated_entities(&before)?);
        check!(eq; after.journal, before.journal);
        check!(eq; after.receipts, before.receipts);
        check!(eq; after.meta, expected_meta);
        check!(eq; Store::format(dir.path())?.lifecycle_revision, None);
        Ok(())
    })
}

#[test]
fn missing_historical_lifecycle_revision_opens_and_migrates_as_a_byte_identical_noop() -> TestResult
{
    let dir = tempfile::tempdir()?;
    let store = seed_retained(dir.path(), expected_time())?;
    metadata(&store, &[("schema:lifecycle_revision", None)])?;
    drop(store);
    let before = retained_bytes(dir.path())?;
    let store = Store::open_with(dir.path(), CACHE_BYTES)?;
    check!(eq; store.walk_table(Table::Performances)?.rows, 1);
    drop(store);
    check!(eq; retained_bytes(dir.path())?, before);
    let report = Store::migrate_with(dir.path(), CACHE_BYTES)?;
    check!(eq; (report.from_schema, report.to_schema, report.to_key_format, report.rewritten_rows, report.dropped_rows, report.already_current, report.integrity_ok), (Some(2), 2, 1, 0, 0, true, true));
    check!(eq; retained_bytes(dir.path())?, before);
    check!(eq; Store::format(dir.path())?.lifecycle_revision, None);
    Ok(())
}
