use super::*;

#[test]
fn current_qualification_v3_preserves_one_identity_across_capabilities() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path();
    let declaration = Store::create_current_qualification_v3(root)?;
    let (store, writer) = Store::open_current_qualification_v3(root)?.into_parts();
    check!(eq; writer.store_identity(), declaration.store_identity());
    check!(eq; writer.protocol(), CaptureProtocolVersion::V2);
    drop((store, writer));
    let (store, writer) = Store::open_public_identity_qualification_v3(root)?.into_parts();
    check!(eq; writer.store_identity(), declaration.store_identity());
    check!(eq; store.store_identity()?, declaration.store_identity().clone());
    Ok(())
}

#[test]
fn current_qualification_v3_capture_survives_public_identity_reopen() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path();
    Store::create_current_qualification_v3(root)?;
    let (store, writer) = Store::open_current_qualification_v3(root)?.into_parts();
    store.bind_run(&RunManifest {
        store_identity: writer.store_identity().as_str().to_string(),
        run: CensusRun::new(SchoolYear::DEFAULT, 1).ok_or("run")?,
        cohort: GradYear::CO2027,
        jurisdictions: vec![UsJurisdiction::Michigan],
    })?;
    let owner = owner(&store, &writer, 2026, CaptureCacheIntent::AcquireNew)?;
    let reference = retained(&store, &writer, &owner, 200)?;
    drop((owner, writer, store));
    let (store, _writer) = Store::open_public_identity_qualification_v3(root)?.into_parts();
    let mut budget = AcquisitionReadBudget::new(128, 8 * 1024 * 1024, 64 * 1024 * 1024)?;
    let captured = store.read_retained_acquisition_v1(&reference, &mut budget)?;
    check!(eq; captured.body_reference(), &reference);
    check!(eq; std::fs::read(captured.body_path())?, b"retained body");
    Ok(())
}

#[test]
fn current_qualification_v3_does_not_create_an_absent_root() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let absent = temporary.path().join("absent");
    check!(Store::open_current_qualification_v3(&absent).is_err());
    check!(!absent.exists());
    Ok(())
}

#[test]
fn current_qualification_v3_does_not_enroll_identity_only_roots() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path();
    Store::create_public_identity_qualification_v3(root)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(root)?.into_parts();
    let before = rows(&store)?;
    drop((store, writer));
    check!(matches!(
        Store::open_current_qualification_v3(root),
        Err(StoreError::Capture(CaptureError::CaptureCapabilityMissing))
    ));
    let (store, _writer) = Store::open_public_identity_qualification_v3(root)?.into_parts();
    check!(eq; rows(&store)?, before);
    Ok(())
}

#[test]
fn current_qualification_v3_does_not_adopt_historical_capture_roots() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path();
    let original = Store::create_capture_qualification_v1(root)?;
    let before = Store::format(root)?;
    check!(matches!(
        Store::open_current_qualification_v3(root),
        Err(StoreError::PublicIdentity(
            crate::PublicIdentityError::OptInRequired
        ))
    ));
    check!(eq; Store::format(root)?, before);
    check!(eq; Store::capture_qualification_control_existing(root)?.show()?,
        CaptureQualification::V1(original));
    Ok(())
}
