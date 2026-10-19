use super::*;
use crate::Application;
use census_domain::roster_affiliation_authority::RosterAuthorityTextV1;
use chrono::DateTime;

fn roster_review(
) -> Result<crate::roster_source_issuance::RosterEnrollmentReviewV1, Box<dyn std::error::Error>> {
    Ok(crate::roster_source_issuance::RosterEnrollmentReviewV1 {
        operation: RosterAuthorityTextV1::parse("current-roster-enrollment")?,
        actor: crate::roster_source_issuance::RosterReviewActorV1::LocalOperator,
        reviewed_at: DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")?.to_utc(),
        rationale: RosterAuthorityTextV1::parse("Explicit current composite roster enrollment")?,
    })
}

fn admitted(
    root: &std::path::Path,
) -> TestResult<census_domain::public_identity_qualification::PublicIdentityQualificationV3> {
    let declaration = Store::create_current_qualification_v3(root)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(root)?.into_parts();
    writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
    Ok(declaration)
}

#[test]
fn current_roster_enrollment_preserves_both_capabilities_and_original_receipt() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path();
    let declaration = admitted(root)?;
    let (store, writer) = Store::open_current_qualification_v3(root)?.into_parts();
    check!(eq; writer.store_identity(), declaration.store_identity());
    let inputs = store.inspect_roster_source_enrollment_v1()?;
    let prepared = store.prepare_roster_source_enrollment_v1(inputs, roster_review()?)?;
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    let first = store.apply_roster_source_enrollment_v1(prepared, &approval, &digest)?;
    check!(first.written());
    check!(eq; store.store_format()?.lifecycle_revision, Some(8));
    let before = rows(&store)?;
    drop((store, writer));
    let (store, writer) = Store::open_public_identity_qualification_v3(root)?.into_parts();
    check!(eq; writer.store_identity(), declaration.store_identity());
    check!(eq; rows(&store)?, before);
    drop((store, writer));
    let (store, writer) = Store::open_current_qualification_v3(root)?.into_parts();
    check!(eq; writer.protocol(), CaptureProtocolVersion::V2);
    check!(eq; writer.store_identity(), declaration.store_identity());
    let repeated =
        store.replay_roster_source_enrollment_v1("current-roster-enrollment", &digest)?;
    check!(matches!(repeated, Application::Repeated(_)));
    check!(eq; first.receipt(), repeated.receipt());
    check!(eq; rows(&store)?, before);
    Ok(())
}

#[test]
fn current_roster_enrollment_cannot_be_replaced_by_different_operation() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path();
    admitted(root)?;
    let (store, _writer) = Store::open_current_qualification_v3(root)?.into_parts();
    let inputs = store.inspect_roster_source_enrollment_v1()?;
    let original_prepared = store.prepare_roster_source_enrollment_v1(inputs, roster_review()?)?;
    let approval = original_prepared.approval_bytes().to_vec();
    let digest = original_prepared.approval_digest().to_owned();
    store.apply_roster_source_enrollment_v1(original_prepared, &approval, &digest)?;
    let before = rows(&store)?;
    let changed_inputs = store.inspect_roster_source_enrollment_v1()?;
    let different_review = crate::roster_source_issuance::RosterEnrollmentReviewV1 {
        operation: RosterAuthorityTextV1::parse("different-operation")?,
        actor: crate::roster_source_issuance::RosterReviewActorV1::LocalOperator,
        reviewed_at: DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")?.to_utc(),
        rationale: RosterAuthorityTextV1::parse("Should not create second enrollment")?,
    };
    let changed_prepared =
        store.prepare_roster_source_enrollment_v1(changed_inputs, different_review)?;
    let changed_approval = changed_prepared.approval_bytes().to_vec();
    let changed_digest = changed_prepared.approval_digest().to_owned();
    check!(store
        .apply_roster_source_enrollment_v1(changed_prepared, &changed_approval, &changed_digest)
        .is_err());
    check!(eq; rows(&store)?, before);
    Ok(())
}
