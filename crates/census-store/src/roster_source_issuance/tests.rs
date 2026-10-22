use super::*;
use crate::{Application, Store};
use census_domain::roster_affiliation_authority::RosterAuthorityTextV1;
use chrono::DateTime;

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

fn command(
    store: &Store,
) -> Result<RosterEnrollmentCommandV1, Box<dyn std::error::Error + Send + Sync>> {
    let view = store.snapshot();
    let revision = view.revision();
    Ok(RosterEnrollmentCommandV1 {
        version: 1,
        store: view.store_identity()?,
        operation: RosterAuthorityTextV1::parse("controlled-enrollment")?,
        actor: RosterReviewActorV1::LocalOperator,
        reviewed_at: DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")?.to_utc(),
        rationale: RosterAuthorityTextV1::parse("Controlled test approval only")?,
        evidence_generation: revision.evidence_generation(),
        derived_generation: revision.derived_generation(),
        input_token_digest: RosterAuthorityTextV1::parse(&codec::digest(
            &view.attachment_input_revision_bytes()?,
        ))?,
    })
}

#[test]
fn controlled_enrollment_is_explicit_atomic_and_replayable() -> TestResult {
    let (_directory, store, _writer) = crate::public_identity_tests::support::fixture()?;
    let command = command(&store)?;
    let original_token = store.snapshot().attachment_input_revision_bytes()?;
    let prepared = store.prepare_roster_source_enrollment_command_v1(
        store.snapshot().revision(),
        command.clone(),
    )?;
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    check!(eq; store.store_format()?.lifecycle_revision, Some(7));
    let first = store.apply_roster_source_enrollment_v1(prepared, &approval, &digest)?;
    check!(first.written());
    check!(eq; store.store_format()?.lifecycle_revision, Some(8));
    check!(eq; store.snapshot().attachment_input_revision_bytes()?, original_token);
    let prepared =
        store.prepare_roster_source_enrollment_command_v1(store.snapshot().revision(), command)?;
    let repeated = store.apply_roster_source_enrollment_v1(prepared, &approval, &digest)?;
    check!(matches!(repeated, Application::Repeated(_)));
    check!(eq; first.receipt(), repeated.receipt());
    Ok(())
}

#[test]
fn changed_approval_and_foreign_revision_do_not_enroll() -> TestResult {
    let (_directory, store, _writer) = crate::public_identity_tests::support::fixture()?;
    let (_other_directory, other, _other_writer) =
        crate::public_identity_tests::support::fixture()?;
    let command = command(&store)?;
    check!(store
        .prepare_roster_source_enrollment_command_v1(other.snapshot().revision(), command.clone())
        .is_err());
    let prepared =
        store.prepare_roster_source_enrollment_command_v1(store.snapshot().revision(), command)?;
    let digest = prepared.approval_digest().to_owned();
    check!(store
        .apply_roster_source_enrollment_v1(prepared, b"unreviewed", &digest)
        .is_err());
    check!(eq; store.store_format()?.lifecycle_revision, Some(7));
    Ok(())
}

#[test]
fn startup_budget_is_not_reset_after_exhaustion() -> TestResult {
    let mut budget = RosterStartupValidationBudgetV1::new();
    check!(budget
        .charge(
            usize::try_from(
                census_domain::roster_affiliation_authority::MAX_ROSTER_PROTECTED_BYTES,
            )?
            .checked_add(1)
            .ok_or("budget size overflow")?
        )
        .is_err());
    check!(budget.charge(0).is_err());
    Ok(())
}

#[test]
fn new_namespace_and_receipts_refuse_generic_writers() -> TestResult {
    check!(refuse_generic_phase("roster_source_issuance").is_err());
    check!(refuse_generic_phase("roster_source_issuance/v1/unknown").is_err());
    check!(refuse_generic_operation("roster_source_issuance_v1:unknown").is_err());
    check!(refuse_generic_phase("unrelated").is_ok());
    Ok(())
}

#[test]
fn copied_foreign_enrollment_cannot_supply_store_origin() -> TestResult {
    use fjall::Readable;
    let (_directory, store, _writer) = crate::public_identity_tests::support::fixture()?;
    let (_other_directory, other, _other_writer) =
        crate::public_identity_tests::support::fixture()?;
    let prepared = store.prepare_roster_source_enrollment_command_v1(
        store.snapshot().revision(),
        command(&store)?,
    )?;
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    let applied = store.apply_roster_source_enrollment_v1(prepared, &approval, &digest)?;
    let snapshot = store.db.snapshot();
    let mut batch = other.db.batch();
    [ENROLLMENT, HEAD]
        .into_iter()
        .try_for_each(|key| -> TestResult {
            let value = snapshot
                .get(&store.meta, key)?
                .ok_or("missing fixture metadata")?;
            batch.insert(&other.meta, key, value);
            Ok(())
        })?;
    let receipt = snapshot
        .get(&store.receipts, &applied.receipt().operation)?
        .ok_or("missing fixture receipt")?;
    batch.insert(
        &other.receipts,
        applied.receipt().operation.as_bytes(),
        receipt,
    );
    crate::meta::put_text(
        &mut batch,
        &other.meta,
        crate::format::LIFECYCLE_REVISION_KEY,
        "8",
    );
    batch.commit()?;
    check!(validate_format(
        &other.db.snapshot(),
        &other.meta,
        &other.journal,
        &other.receipts,
        &other.store_format()?,
        &mut RosterStartupValidationBudgetV1::new()
    )
    .is_err());
    Ok(())
}

#[test]
fn lifecycle_seven_refuses_unexpected_protected_phase_rows() -> TestResult {
    let (_directory, store, _writer) = crate::public_identity_tests::support::fixture()?;
    let mut batch = store.db.batch();
    batch.insert(&store.journal, b"roster_source_issuance\0unexpected", b"{}");
    batch.commit()?;
    check!(validate_format(
        &store.db.snapshot(),
        &store.meta,
        &store.journal,
        &store.receipts,
        &store.store_format()?,
        &mut RosterStartupValidationBudgetV1::new()
    )
    .is_err());
    Ok(())
}

#[test]
fn enrolled_store_cold_reopens_with_original_receipt_and_input_token() -> TestResult {
    let (directory, store, writer) = crate::public_identity_tests::support::fixture()?;
    let command = command(&store)?;
    let token = store.snapshot().attachment_input_revision_bytes()?;
    let prepared = store.prepare_roster_source_enrollment_command_v1(
        store.snapshot().revision(),
        command.clone(),
    )?;
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    let first = store.apply_roster_source_enrollment_v1(prepared, &approval, &digest)?;
    let before = crate::public_identity_tests::support::retained_state(&store)?;
    drop(writer);
    drop(store);
    let (store, _writer) =
        Store::open_public_identity_qualification_v3(directory.path().join("identity"))?
            .into_parts();
    check!(eq; store.snapshot().attachment_input_revision_bytes()?, token);
    check!(eq; crate::public_identity_tests::support::retained_state(&store)?, before);
    let prepared =
        store.prepare_roster_source_enrollment_command_v1(store.snapshot().revision(), command)?;
    let repeated = store.apply_roster_source_enrollment_v1(prepared, &approval, &digest)?;
    check!(eq; first.receipt(), repeated.receipt());
    Ok(())
}

#[test]
fn changed_operation_does_not_create_second_enrollment() -> TestResult {
    let (_directory, store, _writer) = crate::public_identity_tests::support::fixture()?;
    let mut command = command(&store)?;
    let prepared = store.prepare_roster_source_enrollment_command_v1(
        store.snapshot().revision(),
        command.clone(),
    )?;
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    store.apply_roster_source_enrollment_v1(prepared, &approval, &digest)?;
    let before = crate::public_identity_tests::support::retained_state(&store)?;
    command.operation = RosterAuthorityTextV1::parse("different-controlled-operation")?;
    let prepared =
        store.prepare_roster_source_enrollment_command_v1(store.snapshot().revision(), command)?;
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    check!(store
        .apply_roster_source_enrollment_v1(prepared, &approval, &digest)
        .is_err());
    check!(eq; crate::public_identity_tests::support::retained_state(&store)?, before);
    Ok(())
}

#[test]
fn partial_enrollment_cannot_be_recovered_as_admitted() -> TestResult {
    let (_directory, store, _writer) = crate::public_identity_tests::support::fixture()?;
    let prepared = store.prepare_roster_source_enrollment_command_v1(
        store.snapshot().revision(),
        command(&store)?,
    )?;
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    store.apply_roster_source_enrollment_v1(prepared, &approval, &digest)?;
    let mut batch = store.db.batch();
    batch.remove(&store.meta, HEAD);
    batch.commit()?;
    check!(validate_format(
        &store.db.snapshot(),
        &store.meta,
        &store.journal,
        &store.receipts,
        &store.store_format()?,
        &mut RosterStartupValidationBudgetV1::new()
    )
    .is_err());
    Ok(())
}

#[path = "tests/public_api.rs"]
mod public_api;

#[path = "tests/inherited_enrollment.rs"]
mod inherited_enrollment;
