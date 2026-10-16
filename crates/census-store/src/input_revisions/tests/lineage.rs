use super::*;
use census_domain::model::{CensusRun, GradYear, RunManifest, SchoolYear};

fn bind(store: &Store, revision: u32) -> TestResult<RunManifest> {
    let manifest = RunManifest {
        store_identity: store.store_identity()?.as_str().to_owned(),
        run: CensusRun::new(SchoolYear::new(2026).ok_or("school year")?, revision)
            .ok_or("run revision")?,
        cohort: GradYear::CO2027,
        jurisdictions: vec![census_domain::UsJurisdiction::Michigan],
    };
    store.bind_run(&manifest)?;
    Ok(manifest)
}

#[test]
fn independent_stores_can_share_tokens_and_content_roots_without_sharing_lineage() -> TestResult {
    let directory = tempfile::tempdir()?;
    let first = Store::open(directory.path().join("first"))?;
    let second = Store::open(directory.path().join("second"))?;
    let first_run = bind(&first, 1)?;
    let second_run = bind(&second, 2)?;
    enroll(&first)?;
    enroll(&second)?;
    mutate(&first, Table::Athletes, "same-athlete")?;
    mutate(&second, Table::Athletes, "same-athlete")?;
    let first_view = first.snapshot();
    let second_view = second.snapshot();
    check!(eq; enabled_snapshot(&first_view)?, enabled_snapshot(&second_view)?);
    check!(ne; first_view.store_identity()?, second_view.store_identity()?);
    check!(eq; first_view.tables_digest(&TABLES)?, second_view.tables_digest(&TABLES)?);
    check!(eq; first_view.run_manifest()?, Some(first_run));
    check!(eq; second_view.run_manifest()?, Some(second_run));
    check!(ne; first_view.run_manifest()?, second_view.run_manifest()?);
    let before = retained_state(&second)?;
    check!(matches!(
        second.enroll_public_identity_input_revisions_v2(first_view.revision()),
        Err(StoreError::RevisionOriginMismatch { .. })
    ));
    check!(eq; retained_state(&second)?, before);
    Ok(())
}

#[test]
fn restored_divergent_stores_can_share_lineage_and_tokens_but_require_distinct_content_roots(
) -> TestResult {
    let directory = tempfile::tempdir()?;
    let original = directory.path().join("original");
    let backup = directory.path().join("backup");
    let first_root = directory.path().join("first");
    let second_root = directory.path().join("second");
    let store = Store::open(&original)?;
    let identity = store.store_identity()?;
    let run = bind(&store, 1)?;
    enroll(&store)?;
    drop(store);
    Store::backup(&original, &backup)?;
    Store::restore(&backup, &first_root)?;
    Store::restore(&backup, &second_root)?;
    let first = Store::open_existing(&first_root)?;
    let second = Store::open_existing(&second_root)?;
    first.append(Table::Athletes, &derived("same-id", 1))?;
    second.append(Table::Athletes, &derived("same-id", 2))?;
    let first_view = first.snapshot();
    let second_view = second.snapshot();
    check!(eq; first_view.store_identity()?, identity);
    check!(eq; second_view.store_identity()?, identity);
    check!(eq; first_view.run_manifest()?, Some(run.clone()));
    check!(eq; second_view.run_manifest()?, Some(run));
    check!(eq; enabled_snapshot(&first_view)?, enabled_snapshot(&second_view)?);
    check!(ne; first_view.tables_digest(&TABLES)?, second_view.tables_digest(&TABLES)?);
    check!(ne; serde_json::to_value(first_view.scan::<DerivedRow>(Table::Athletes)?)?, serde_json::to_value(second_view.scan::<DerivedRow>(Table::Athletes)?)?);
    Ok(())
}
