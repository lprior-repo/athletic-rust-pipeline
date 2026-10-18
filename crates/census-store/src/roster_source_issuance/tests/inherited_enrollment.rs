use super::*;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::result_school_correspondence::{ResultSchoolReviewActorV1, ResultSchoolTextV1};

fn roster_review() -> Result<RosterEnrollmentReviewV1, Box<dyn std::error::Error + Send + Sync>> {
    Ok(RosterEnrollmentReviewV1 {
        operation: RosterAuthorityTextV1::parse("inherited-enrollment-replay")?,
        actor: RosterReviewActorV1::LocalOperator,
        reviewed_at: DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")?.to_utc(),
        rationale: RosterAuthorityTextV1::parse("Preserve the original roster enrollment receipt")?,
    })
}

fn school_enroll(store: &Store) -> TestResult {
    let inputs = store.inspect_result_school_enrollment_v1(AttachmentReplayGrantV1::new())?;
    let prepared = store.prepare_result_school_enrollment_v1(
        inputs,
        crate::ResultSchoolEnrollmentReviewV1 {
            operation: ResultSchoolTextV1::parse("school-inherited-enrollment")?,
            actor: ResultSchoolReviewActorV1::LocalOperator,
            reviewed_at: DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")?.to_utc(),
            rationale: ResultSchoolTextV1::parse(
                "Explicit enrollment preserves the prior authority",
            )?,
        },
    )?;
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    check!(store
        .apply_result_school_enrollment_v1(prepared, &approval, &digest)?
        .written());
    Ok(())
}

#[test]
fn inherited_eight_replays_original_enrollment_without_new_writes() -> TestResult {
    let (directory, store, writer) = crate::public_identity_tests::support::fixture()?;
    let inputs = store.inspect_roster_source_enrollment_v1()?;
    let prepared = store.prepare_roster_source_enrollment_v1(inputs, roster_review()?)?;
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    let original = store.apply_roster_source_enrollment_v1(prepared, &approval, &digest)?;
    check!(original.written());
    school_enroll(&store)?;
    drop(writer);
    drop(store);
    let (store, _writer) =
        Store::open_public_identity_qualification_v3(directory.path().join("identity"))?
            .into_parts();
    check!(eq; store.store_format()?.lifecycle_revision, Some(9));
    let before = crate::public_identity_tests::support::retained_state(&store)?;
    let inputs = store.inspect_roster_source_enrollment_v1()?;
    let prepared = store.prepare_roster_source_enrollment_v1(inputs, roster_review()?)?;
    check!(eq; prepared.approval_bytes(), approval);
    let repeated = store.apply_roster_source_enrollment_v1(prepared, &approval, &digest)?;
    check!(repeated.repeated());
    check!(eq; repeated.receipt(), original.receipt());
    check!(eq; crate::public_identity_tests::support::retained_state(&store)?, before);
    Ok(())
}

#[test]
fn identity_only_nine_cannot_apply_a_prepared_roster_enrollment() -> TestResult {
    let (_directory, store, _writer) = crate::public_identity_tests::support::fixture()?;
    let inputs = store.inspect_roster_source_enrollment_v1()?;
    let prepared = store.prepare_roster_source_enrollment_v1(inputs, roster_review()?)?;
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    school_enroll(&store)?;
    let before = crate::public_identity_tests::support::retained_state(&store)?;
    check!(matches!(
        store.apply_roster_source_enrollment_v1(prepared, &approval, &digest),
        Err(crate::StoreError::RosterSourceIssuance(
            RosterSourceIssuanceStoreError::EnrollmentRequired
        ))
    ));
    check!(matches!(
        store.inspect_roster_source_enrollment_v1(),
        Err(crate::StoreError::RosterSourceIssuance(
            RosterSourceIssuanceStoreError::EnrollmentRequired
        ))
    ));
    check!(eq; crate::public_identity_tests::support::retained_state(&store)?, before);
    check!(eq; store.store_format()?.lifecycle_revision, Some(9));
    Ok(())
}
