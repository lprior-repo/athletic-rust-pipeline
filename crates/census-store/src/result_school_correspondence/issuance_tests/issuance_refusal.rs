use super::*;

#[test]
fn fresh_lifecycle_two_cannot_inspect_school_authority() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    check!(eq; store.store_format()?.lifecycle_revision, Some(2));
    let before = retained_state(&store)?;
    check!(matches!(
        store.inspect_result_school_enrollment_v1(AttachmentReplayGrantV1::new()),
        Err(StoreError::ResultSchoolCorrespondence(
            Error::EnrollmentRequired
        ))
    ));
    check!(matches!(
        store.inspect_result_school_correspondence_v1(AttachmentReplayGrantV1::new()),
        Err(StoreError::ResultSchoolCorrespondence(
            Error::EnrollmentRequired
        ))
    ));
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn stray_issuance_record_blocks_inspection_and_cold_start_without_mutation() -> TestResult {
    let (directory, store, writer) = enrolled()?;
    store
        .journal
        .insert("result_school_correspondence/v1/record/00000000", b"{}")?;
    let before = retained_state(&store)?;
    check!(matches!(
        store.inspect_result_school_correspondence_v1(AttachmentReplayGrantV1::new()),
        Err(StoreError::ResultSchoolCorrespondence(Error::Corrupt(
            "extra issuance namespace row"
        )))
    ));
    check!(eq; retained_state(&store)?, before);
    store.flush()?;
    drop(writer);
    drop(store);
    let root = directory.path().join("identity");
    check!(matches!(
        Store::open_public_identity_qualification_v3(&root),
        Err(StoreError::ResultSchoolCorrespondence(Error::Corrupt(
            "extra issuance namespace row"
        )))
    ));
    check!(eq; crate::public_identity_tests::support::physical_state(&root)?, before);
    Ok(())
}
