use super::super::ResultSchoolCorrespondenceStoreError as Error;
use super::*;
use crate::StoreError;

#[test]
fn foreign_and_stale_inspections_preserve_every_keyspace() -> TestResult {
    let (_directory, store, _writer) = fixture()?;
    let (_other_directory, other, _other_writer) = fixture()?;
    let foreign = other.inspect_result_school_enrollment_v1(AttachmentReplayGrantV1::new())?;
    let before = retained_state(&store)?;
    let other_before = retained_state(&other)?;
    check!(matches!(
        store.prepare_result_school_enrollment_v1(foreign, review()?),
        Err(StoreError::RevisionOriginMismatch { .. })
    ));
    check!(eq; retained_state(&store)?, before);
    check!(eq; retained_state(&other)?, other_before);
    let foreign = other.inspect_result_school_enrollment_v1(AttachmentReplayGrantV1::new())?;
    let prepared = other.prepare_result_school_enrollment_v1(foreign, review()?)?;
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    check!(matches!(
        store.apply_result_school_enrollment_v1(prepared, &approval, &digest),
        Err(StoreError::RevisionOriginMismatch { .. })
    ));
    check!(eq; retained_state(&store)?, before);
    check!(eq; retained_state(&other)?, other_before);
    let inputs = store.inspect_result_school_enrollment_v1(AttachmentReplayGrantV1::new())?;
    predecessor::advance(&store)?;
    let before = retained_state(&store)?;
    check!(matches!(
        store.prepare_result_school_enrollment_v1(inputs, review()?),
        Err(StoreError::RevisionMoved { .. })
    ));
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn stale_prepared_and_independent_approval_mismatches_are_atomic() -> TestResult {
    for mismatch in 0..3 {
        let (_directory, store, _writer) = fixture()?;
        predecessor::enroll_roster(&store)?;
        let inputs = store.inspect_result_school_enrollment_v1(AttachmentReplayGrantV1::new())?;
        let prepared = store.prepare_result_school_enrollment_v1(inputs, review()?)?;
        let mut approval = prepared.approval_bytes().to_vec();
        let mut digest = prepared.approval_digest().clone();
        match mismatch {
            0 => predecessor::advance(&store)?,
            1 => approval.push(b' '),
            _ => digest = ResultSchoolDigestV1::parse(&"0".repeat(64))?,
        }
        let before = retained_state(&store)?;
        let result = store.apply_result_school_enrollment_v1(prepared, &approval, &digest);
        if mismatch == 0 {
            check!(matches!(result, Err(StoreError::RevisionMoved { .. })));
        } else {
            check!(matches!(
                result,
                Err(StoreError::ResultSchoolCorrespondence(
                    Error::ApprovalMismatch
                ))
            ));
        }
        check!(eq; retained_state(&store)?, before);
        check!(eq; store.store_format()?.lifecycle_revision, Some(8));
    }
    Ok(())
}

#[test]
fn roster_enrollment_invalidates_inspected_and_prepared_seven_capability() -> TestResult {
    for prepared_first in [false, true] {
        let (_directory, store, _writer) = fixture()?;
        let inputs = store.inspect_result_school_enrollment_v1(AttachmentReplayGrantV1::new())?;
        if prepared_first {
            let prepared = store.prepare_result_school_enrollment_v1(inputs, review()?)?;
            let approval = prepared.approval_bytes().to_vec();
            let digest = prepared.approval_digest().clone();
            predecessor::enroll_roster(&store)?;
            let before = retained_state(&store)?;
            check!(matches!(
                store.apply_result_school_enrollment_v1(prepared, &approval, &digest),
                Err(StoreError::ResultSchoolCorrespondence(Error::InputMoved))
            ));
            check!(eq; retained_state(&store)?, before);
        } else {
            predecessor::enroll_roster(&store)?;
            let before = retained_state(&store)?;
            check!(matches!(
                store.prepare_result_school_enrollment_v1(inputs, review()?),
                Err(StoreError::ResultSchoolCorrespondence(Error::InputMoved))
            ));
            check!(eq; retained_state(&store)?, before);
        }
        check!(eq; store.store_format()?.lifecycle_revision, Some(8));
    }
    Ok(())
}
