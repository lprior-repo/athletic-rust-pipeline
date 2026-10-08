use super::*;

#[test]
fn schema1_preserves_current_derived_generation_counters_receipts_and_raw_observations(
) -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    store.append(
        Table::Performances,
        &performance("perf_historic", json!(1094)),
    )?;
    let source = census_domain::model::SourceSchoolObservation::new(
        census_domain::model::SourceNamespace::MilesplitTeam,
        "4",
        "https://example.test/team/4",
        "Historic",
        "2026-05-01",
    );
    let source_id = source.id.clone();
    let mut raw = serde_json::to_value(census_domain::model::SourceObservation::School(source))?;
    raw.as_object_mut().ok_or("raw source object")?.insert(
        "provider".into(),
        json!({"TimeSeconds":1094,"mark":"10.941"}),
    );
    store.append(Table::SourceObservations, &raw)?;
    store.append(
        Table::Events,
        &json!({"id":"evt_historic","meet":"meet_historic",
        "kind":"Track100m","gender":"Female","source_labels":[],"evidence":[]}),
    )?;
    let coverage = CoverageRow::new(CoverageScope::Jurisdiction, "wi");
    let mut batch = store.db.batch();
    batch.insert(
        &store.entities,
        derived_key(Table::Coverage, 7, coverage.id.as_bytes()),
        serde_json::to_vec(&coverage)?,
    );
    crate::rows::put_row_mark(&mut batch, &store.meta, Table::Coverage, 1);
    generation::write_seed(&mut batch, &store.meta, 55, 7, 12, 3);
    meta::put_text(&mut batch, &store.meta, "schema:store_version", "1");
    batch.commit()?;
    let mut effect = store.write_batch();
    effect.journal_done(
        "milesplit_owned_capture_v1",
        "raw/manifest",
        &json!({"TimeSeconds":1094,"body":"10.941","capture_sha256":"immutable"}),
    )?;
    effect.commit_once("historic-effect", "historic-digest")?;
    let before_meta = all_bytes(&store.meta)?;
    let before_receipts = all_bytes(&store.receipts)?;
    let before_journal = all_bytes(&store.journal)?;
    let before_raw = store
        .entities
        .get(observation_key(Table::SourceObservations, &source_id, 0))?
        .ok_or("raw observation")?
        .to_vec();
    let before_event = store
        .entities
        .get(observation_key(Table::Events, "evt_historic", 0))?
        .ok_or("historic event")?
        .to_vec();
    drop(store);
    Store::migrate(dir.path())?;
    let store = Store::open(dir.path())?;
    check!(eq; (store.evidence_generation(), store.derived_generation()), (55, 7));
    check!(eq; store.scan::<CoverageRow>(Table::Coverage)?, vec![coverage]);
    check!(eq; all_bytes(&store.receipts)?, before_receipts);
    check!(eq; all_bytes(&store.journal)?, before_journal);
    check!(eq; store.entities.get(observation_key(Table::SourceObservations, &source_id, 0))?.ok_or("retained raw")?.to_vec(), before_raw);
    check!(eq; store.entities.get(observation_key(Table::Events, "evt_historic", 0))?.ok_or("retained historic event")?.to_vec(), before_event);
    let mut expected_meta = before_meta;
    expected_meta.insert(b"schema:store_version".to_vec(), b"2".to_vec());
    check!(eq; all_bytes(&store.meta)?, expected_meta);
    Ok(())
}

#[test]
fn schema0_upgrades_keys_and_generations_before_exacttime_conversion() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    store.append(
        Table::Performances,
        &performance("perf_historic", json!(1094)),
    )?;
    erase_current_schema(&store)?;
    drop(store);
    let coverage = CoverageRow::new(CoverageScope::Jurisdiction, "wi");
    raw_import(
        dir.path(),
        Table::Coverage,
        &[(&coverage.id, 0, coverage.clone())],
    )?;
    let report = Store::migrate(dir.path())?;
    check!(eq; (report.from_schema, report.to_schema, report.rewritten_rows), (None, 2, 2));
    let store = Store::open(dir.path())?;
    check!(eq; store.derived_generation(), 1);
    check!(eq; store.scan::<CoverageRow>(Table::Coverage)?, vec![coverage]);
    let row = store
        .scan::<CanonicalPerformance>(Table::Performances)?
        .into_iter()
        .next()
        .ok_or("canonical row")?;
    check!(eq; row.event.as_str(), "evt_historic");
    check!(eq; row.id.as_str(), "perf_historic");
    check!(eq; serde_json::to_value(row.mark)?, json!({"TimeSeconds":expected_time()}));
    Ok(())
}

#[test]
fn versioned_empty_schema1_does_not_reinitialize_generations_or_creation_lineage() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let mut batch = store.db.batch();
    generation::write_seed(&mut batch, &store.meta, 23, 7, 12, 3);
    meta::put_text(&mut batch, &store.meta, "schema:store_version", "1");
    batch.commit()?;
    let before = all_bytes(&store.meta)?;
    drop(store);
    check!(matches!(
        Store::open(dir.path()),
        Err(StoreError::MigrationRequired { .. })
    ));
    Store::migrate(dir.path())?;
    let store = Store::open(dir.path())?;
    let mut expected = before;
    expected.insert(b"schema:store_version".to_vec(), b"2".to_vec());
    check!(eq; all_bytes(&store.meta)?, expected);
    Ok(())
}

#[test]
fn explicitly_versioned_schema0_preserves_creation_lineage_during_the_two_stage_upgrade(
) -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    store.append(
        Table::Performances,
        &performance("perf_historic", json!(1094)),
    )?;
    let before = super::super::super::read_format(&store.meta)?;
    let mut batch = store.db.batch();
    meta::put_text(&mut batch, &store.meta, "schema:store_version", "0");
    meta::put_text(&mut batch, &store.meta, "schema:key_format", "0");
    batch.commit()?;
    drop(store);
    let coverage = CoverageRow::new(CoverageScope::Jurisdiction, "wi");
    raw_import(
        dir.path(),
        Table::Coverage,
        &[(&coverage.id, 0, coverage.clone())],
    )?;
    let report = Store::migrate(dir.path())?;
    check!(eq; (report.from_schema, report.to_schema, report.to_key_format, report.rewritten_rows), (Some(0), 2, 1, 2));
    let after = Store::format(dir.path())?;
    check!(eq; (after.created_by, after.created_at), (before.created_by, before.created_at));
    let store = Store::open(dir.path())?;
    check!(eq; store.derived_generation(), 1);
    check!(eq; store.scan::<CoverageRow>(Table::Coverage)?, vec![coverage]);
    let row = store
        .scan::<CanonicalPerformance>(Table::Performances)?
        .into_iter()
        .next()
        .ok_or("canonical row")?;
    check!(eq; serde_json::to_value(row.mark)?, json!({"TimeSeconds":expected_time()}));
    Ok(())
}
