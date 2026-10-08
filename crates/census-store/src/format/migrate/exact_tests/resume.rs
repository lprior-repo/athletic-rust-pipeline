use super::*;

#[test]
fn failed_page_preserves_its_cursor_and_resume_visits_the_bad_row_and_later_rows() -> TestResult {
    let dir = tempfile::tempdir()?;
    seed(
        dir.path(),
        &[
            performance("a", json!(1094)),
            performance("b", json!(0)),
            performance("c", json!(1095)),
        ],
    )?;
    check!(matches!(
        Store::migrate(dir.path()),
        Err(StoreError::Refused { .. })
    ));
    let first = raw_performance(dir.path(), "a", 0)?;
    let first_row: Value = serde_json::from_slice(&first)?;
    check!(eq; first_row["mark"]["TimeSeconds"], expected_time());
    let bad: Value = serde_json::from_slice(&raw_performance(dir.path(), "b", 1)?)?;
    let later: Value = serde_json::from_slice(&raw_performance(dir.path(), "c", 2)?)?;
    check!(eq; bad["mark"]["TimeSeconds"], json!(0));
    check!(eq; later["mark"]["TimeSeconds"], json!(1095));
    check!(matches!(
        Store::open(dir.path()),
        Err(StoreError::MigrationRequired { .. })
    ));
    raw_import(
        dir.path(),
        Table::Performances,
        &[("b", 1, performance("b", json!(1096)))],
    )?;
    let report = Store::migrate(dir.path())?;
    check!(eq; (report.rewritten_rows, report.dropped_rows), (3, 0));
    check!(eq; raw_performance(dir.path(), "a", 0)?, first);
    let repaired: Value = serde_json::from_slice(&raw_performance(dir.path(), "b", 1)?)?;
    let last: Value = serde_json::from_slice(&raw_performance(dir.path(), "c", 2)?)?;
    check!(eq; repaired["mark"]["TimeSeconds"], json!({"nanoseconds":10_960_000_000_i64,"precision":2}));
    check!(eq; last["mark"]["TimeSeconds"], json!({"nanoseconds":10_950_000_000_i64,"precision":2}));
    check!(eq; Store::format(dir.path())?.migration_target, None);
    check!(eq; Store::open(dir.path())?.walk_table(Table::Performances)?.rows, 3);
    Ok(())
}

#[test]
fn interruption_after_conversion_before_schema_publication_is_idempotent_on_reopen() -> TestResult {
    let dir = tempfile::tempdir()?;
    seed(dir.path(), &[performance("perf_historic", json!(1094))])?;
    {
        let (db, entities, journal, meta_keyspace, _) =
            crate::open_keyspaces(dir.path(), crate::format::DEFAULT_CACHE_BYTES)?;
        let mut marker = db.batch();
        meta::put_text(&mut marker, &meta_keyspace, "schema:migrating_to", "2");
        super::super::time::commit(marker)?;
        let context = super::super::time::Context {
            db: &db,
            entities: &entities,
            journal: &journal,
            meta: &meta_keyspace,
        };
        let converted = super::super::time::migrate(&context)?;
        check!(eq; converted.rewritten, 1);
    }
    let converted = raw_performance(dir.path(), "perf_historic", 0)?;
    check!(matches!(
        Store::open(dir.path()),
        Err(StoreError::MigrationRequired { .. })
    ));
    let report = Store::migrate(dir.path())?;
    check!(eq; report.rewritten_rows, 1);
    check!(eq; raw_performance(dir.path(), "perf_historic", 0)?, converted);
    let store = Store::open(dir.path())?;
    check!(eq; store.walk_table(Table::Performances)?.rows, 1);
    check!(eq; store.derived_generation(), 0);
    Ok(())
}

#[test]
fn exacttime_accepts_the_positive_integer_limits_without_overflow() -> TestResult {
    let dir = tempfile::tempdir()?;
    seed(
        dir.path(),
        &[
            performance("a", json!(1)),
            performance("b", json!(922_337_203_685_i64)),
        ],
    )?;
    Store::migrate(dir.path())?;
    let first: Value = serde_json::from_slice(&raw_performance(dir.path(), "a", 0)?)?;
    let last: Value = serde_json::from_slice(&raw_performance(dir.path(), "b", 1)?)?;
    check!(eq; first["mark"]["TimeSeconds"], json!({"nanoseconds":10_000_000,"precision":2}));
    check!(eq; last["mark"]["TimeSeconds"], json!({"nanoseconds":9_223_372_036_850_000_000_i64,"precision":2}));
    Ok(())
}
