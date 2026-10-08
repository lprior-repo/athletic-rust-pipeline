use super::*;

#[test]
fn migrating_a_legacy_store_rewrites_derived_rows_and_collapses_maps() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path();
    let store = Store::open(root)?;
    store.append(Table::Schools, &school("Legacy School"))?;
    erase_current_schema(&store)?;
    drop(store);
    let wi = CoverageRow::new(CoverageScope::Jurisdiction, "wi");
    let mn = CoverageRow::new(CoverageScope::Jurisdiction, "mn");
    raw_import(
        root,
        Table::Coverage,
        &[(&wi.id, 0, wi.clone()), (&mn.id, 0, mn.clone())],
    )?;
    let decided = case("case:1", ReviewState::Resolved);
    raw_import(
        root,
        Table::ReviewCases,
        &[
            ("case:1", 3, decided.clone()),
            ("case:1", 7, case("case:1", ReviewState::Pending)),
        ],
    )?;
    let early = access("2026-01-01T00:00:00Z", "first refusal");
    let late = access("2026-02-01T00:00:00Z", "second refusal");
    raw_import(
        root,
        Table::SourceAccess,
        &[(&late.id, 5, early), (&late.id, 9, late.clone())],
    )?;
    let report = Store::migrate(root)?;
    check!(eq; (report.from_schema, report.rewritten_rows, report.dropped_rows), (None, 3, 2));
    check!(report.integrity_ok);
    let store = Store::open(root)?;
    check!(eq; store.scan::<CanonicalSchool>(Table::Schools)?, vec![school("Legacy School")]);
    let mut expected = vec![wi, mn];
    expected.sort_by(|a, b| a.id.cmp(&b.id));
    check!(eq; store.scan::<CoverageRow>(Table::Coverage)?, expected);
    check!(eq; store.scan::<ReviewCase>(Table::ReviewCases)?, vec![decided]);
    check!(eq; store.scan::<SourceAccessCondition>(Table::SourceAccess)?, vec![late]);
    check!(eq; store.walk_table(Table::SourceAccess)?.foreign_sequences, 0);
    check!(eq; store.derived_generation(), 1);
    Ok(())
}

#[test]
fn legacy_failure_resumes_without_losing_a_folded_group() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    erase_current_schema(&store)?;
    drop(store);
    let resolved = case("a", ReviewState::Resolved);
    raw_import(
        dir.path(),
        Table::ReviewCases,
        &[
            ("a", 1, serde_json::to_value(&resolved)?),
            (
                "a",
                2,
                serde_json::to_value(case("a", ReviewState::Pending))?,
            ),
            ("b", 3, serde_json::json!({"id":"b","state":"invalid"})),
        ],
    )?;
    check!(matches!(
        Store::migrate(dir.path()),
        Err(StoreError::Json { .. })
    ));
    check!(eq; Store::format(dir.path())?.migration_target, Some(2));
    raw_import(
        dir.path(),
        Table::ReviewCases,
        &[("b", 3, case("b", ReviewState::Pending))],
    )?;
    let report = Store::migrate(dir.path())?;
    check!(eq; (report.rewritten_rows, report.dropped_rows), (2, 1));
    let store = Store::open(dir.path())?;
    check!(eq; store.scan::<ReviewCase>(Table::ReviewCases)?, vec![resolved, case("b", ReviewState::Pending)]);
    check!(eq; store.walk_table(Table::ReviewCases)?.rows, 2);
    Ok(())
}
