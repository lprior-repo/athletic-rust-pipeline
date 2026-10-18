use super::ResultSchoolCorrespondenceStoreError as Error;
use super::*;
use crate::public_identity_tests::support::{retained_state, TestResult};
use crate::{Application, Store, StoreError};

mod issuance_lifecycle;
mod issuance_refusal;
mod issuance_replay;

fn enrolled() -> TestResult<(tempfile::TempDir, Store, crate::PublicIdentityWriterV1)> {
    let (directory, store, writer) = crate::public_identity_tests::support::fixture()?;
    let inputs = store.inspect_result_school_enrollment_v1(AttachmentReplayGrantV1::new())?;
    let prepared = store.prepare_result_school_enrollment_v1(
        inputs,
        ResultSchoolEnrollmentReviewV1 {
            operation: ResultSchoolTextV1::parse("store-school-boundary-enrollment")?,
            actor: ResultSchoolReviewActorV1::LocalOperator,
            reviewed_at: chrono::DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")?.to_utc(),
            rationale: ResultSchoolTextV1::parse(
                "Controlled enrollment through public protected writer",
            )?,
        },
    )?;
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    check!(matches!(
        store.apply_result_school_enrollment_v1(prepared, &approval, &digest)?,
        Application::Written(_)
    ));
    Ok((directory, store, writer))
}
