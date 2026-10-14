use super::*;
use census_domain::model::{CensusRun, GradYear, RunManifest, SchoolYear};

fn expect_unsupported(result: crate::StoreResult<Store>) -> TestResult {
    match result {
        Err(StoreError::Installation(InstallationError::EnrollmentOnlyUnsupported { .. })) => {
            Ok(())
        }
        Err(other) => Err(format!("wrong execution refusal={other:?}").into()),
        Ok(_) => Err("enrollment-only store opened for execution".into()),
    }
}

#[test]
fn every_generic_open_and_direct_legacy_binding_refuse_enrollment_only() -> TestResult {
    let root = tempfile::tempdir()?;
    let config = prepare(root.path())?;
    let mut control = Store::installation_control_existing(root.path())?;
    control.enroll(&config)?;
    let before = witness(&control.store)?;
    let manifest = RunManifest {
        store_identity: control.store.store_identity()?.as_str().to_string(),
        run: CensusRun::new(SchoolYear::new(2026).ok_or("season")?, 1).ok_or("run")?,
        cohort: GradYear::CO2027,
        jurisdictions: vec![census_domain::UsJurisdiction::Michigan],
    };
    if !matches!(
        control.store.bind_run(&manifest),
        Err(StoreError::Installation(
            InstallationError::EnrollmentOnlyUnsupported { .. }
        ))
    ) {
        return Err("direct legacy bind accepted".into());
    }
    equal(witness(&control.store)?, before.clone())?;
    drop(control);
    expect_unsupported(Store::open(root.path()))?;
    expect_unsupported(Store::open_with(root.path(), 8_388_608))?;
    expect_unsupported(Store::open_existing(root.path()))?;
    let inspected = Store::inspect_with(root.path(), 8_388_608)?;
    equal(inspected.format().lifecycle_revision, Some(3))?;
    drop(inspected);
    if !matches!(
        Store::migrate_with(root.path(), 8_388_608),
        Err(StoreError::Installation(
            InstallationError::EnrollmentOnlyUnsupported { .. }
        ))
    ) {
        return Err("migrate unexpectedly activated enrollment-only store".into());
    }
    let control = Store::installation_control_existing(root.path())?;
    equal(witness(&control.store)?, before.clone())
}

#[test]
fn legacy_fresh_lifecycle_and_single_run_binding_remain_unchanged() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open_with(root.path(), 8_388_608)?;
    equal(store.store_format()?.lifecycle_revision, Some(2))?;
    let manifest = RunManifest {
        store_identity: store.store_identity()?.as_str().to_string(),
        run: CensusRun::new(SchoolYear::new(2026).ok_or("season")?, 1).ok_or("run")?,
        cohort: GradYear::CO2027,
        jurisdictions: vec![census_domain::UsJurisdiction::Michigan],
    };
    store.bind_run(&manifest)?;
    let before = witness(&store)?;
    store.bind_run(&manifest)?;
    equal(witness(&store)?, before.clone())?;
    let mut changed = manifest.clone();
    changed
        .jurisdictions
        .push(census_domain::UsJurisdiction::Ohio);
    if !matches!(store.bind_run(&changed), Err(StoreError::Refused { .. })) {
        return Err("legacy rebind accepted".into());
    }
    equal(store.run_manifest()?, Some(manifest))?;
    equal(witness(&store)?, before)
}

#[test]
fn enrolled_schema_or_key_downgrade_cannot_enter_legacy_migration_or_rewrite_any_record(
) -> TestResult {
    for (key, value) in [
        ("schema:store_version", "1"),
        ("schema:key_format", "0"),
        ("schema:lifecycle_revision", "2"),
    ] {
        let root = tempfile::tempdir()?;
        let config = prepare(root.path())?;
        let mut control = Store::installation_control_existing(root.path())?;
        control.enroll(&config)?;
        control.store.meta.insert(key, value)?;
        control.store.flush()?;
        let before = witness(&control.store)?;
        let before_format = control.store.store_format()?;
        drop(control);
        match Store::migrate_with(root.path(), 8_388_608) {
            Err(StoreError::SchemaUnknown { .. }) if key != "schema:lifecycle_revision" => {}
            Err(StoreError::Installation(InstallationError::MarkerMismatch { found: Some(2) }))
                if key == "schema:lifecycle_revision" => {}
            other => return Err(format!("{key} downgrade migration={other:?}").into()),
        }
        let (db, entities, journal, meta, receipts) =
            crate::open_keyspaces(root.path(), 8_388_608)?;
        equal(crate::format::read_format(&meta)?, before_format)?;
        let after = Witness {
            sequence: db.snapshot().seqno(),
            evidence: crate::meta::get_u64(&meta, "generation:entities")?.ok_or("generation")?,
            derived: crate::meta::get_u64(&meta, "generation:derived:current")?
                .ok_or("generation")?,
            rows: vec![
                rows(&db.snapshot(), &entities)?,
                rows(&db.snapshot(), &journal)?,
                rows(&db.snapshot(), &meta)?,
                rows(&db.snapshot(), &receipts)?,
            ],
        };
        equal(after, before)?;
    }
    Ok(())
}
