use super::fixture::*;
use crate::evidence_subject_attachment::{
    EvidenceSubjectAttachmentPreparationV1 as Preparation,
    EvidenceSubjectAttachmentStoreError as AttachmentError,
};
use crate::public_identity_tests::support::{retained_state, TestResult};
use crate::{StoreError, Table};
use census_domain::evidence_subject_attachment::*;

#[test]
fn each_explicit_approval_component_must_equal_the_prepared_bytes_and_digest() -> TestResult {
    let fixture = Fixture::new()?;
    let context = Context::synthetic("approval")?;
    let before = retained_state(&fixture.store)?;
    for change_bytes in [true, false] {
        let inspection = fixture.inspect()?;
        let command = context.command("approval", &inspection)?;
        let prepared = prepare(&fixture.store, inspection, command, &context)?;
        let mut approval = prepared.approval_bytes().to_vec();
        let mut digest = prepared.approval_digest().clone();
        if change_bytes {
            approval.push(b' ');
        } else {
            digest = AttachmentDigestV1::parse(&"f".repeat(64))?;
            check!(ne; &digest, prepared.approval_digest());
        }
        check!(matches!(
            fixture
                .store
                .apply_evidence_subject_attachment_v1(prepared, &approval, &digest),
            Err(StoreError::EvidenceSubjectAttachment(
                AttachmentError::Corrupt("explicit attachment approval differs")
            ))
        ));
        check!(eq; retained_state(&fixture.store)?, before);
    }
    Ok(())
}

#[test]
fn foreign_inspection_cannot_prepare_against_another_store_and_preserves_both_stores() -> TestResult
{
    let origin = Fixture::new()?;
    let other = Fixture::new()?;
    let context = Context::synthetic("foreign-inspection")?;
    let inspection = origin.inspect()?;
    let command = context.command("foreign-inspection", &inspection)?;
    let before_origin = retained_state(&origin.store)?;
    let before_other = retained_state(&other.store)?;
    check!(
        matches!(other.store.prepare_evidence_subject_attachment_v1(command,
            Preparation::New { inspection, context: &context.borrowed() }),
            Err(StoreError::RevisionOriginMismatch { operation }) if operation == "foreign-inspection"
        )
    );
    check!(eq; retained_state(&origin.store)?, before_origin);
    check!(eq; retained_state(&other.store)?, before_other);
    Ok(())
}

#[test]
fn prepared_attachment_cannot_be_applied_through_a_foreign_store() -> TestResult {
    let origin = Fixture::new()?;
    let other = Fixture::new()?;
    let context = Context::synthetic("foreign-apply")?;
    let inspection = origin.inspect()?;
    let command = context.command("foreign-apply", &inspection)?;
    let prepared = prepare(&origin.store, inspection, command, &context)?;
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    let before_origin = retained_state(&origin.store)?;
    let before_other = retained_state(&other.store)?;
    check!(
        matches!(other.store.apply_evidence_subject_attachment_v1(prepared, &approval, &digest),
            Err(StoreError::RevisionOriginMismatch { operation }) if operation == "foreign-apply"
        )
    );
    check!(eq; retained_state(&origin.store)?, before_origin);
    check!(eq; retained_state(&other.store)?, before_other);
    Ok(())
}

#[test]
fn a_new_replay_only_command_requires_an_inspection_and_never_writes() -> TestResult {
    let fixture = Fixture::new()?;
    let context = Context::synthetic("new-replay-only")?;
    let inspection = fixture.inspect()?;
    let command = context.command("new-replay-only", &inspection)?;
    drop(inspection);
    let before = retained_state(&fixture.store)?;
    check!(matches!(
        fixture
            .store
            .prepare_evidence_subject_attachment_v1(command, Preparation::ReplayOnly),
        Err(StoreError::EvidenceSubjectAttachment(
            AttachmentError::Corrupt("new attachment requires an inspection")
        ))
    ));
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}

#[test]
fn an_existing_operation_rejects_a_changed_command_before_any_fresh_write() -> TestResult {
    let fixture = Fixture::new()?;
    let context = Context::synthetic("operation-conflict")?;
    let inspection = fixture.inspect()?;
    let mut command = context.command("operation-conflict", &inspection)?;
    written(apply(
        &fixture.store,
        prepare(&fixture.store, inspection, command.clone(), &context)?,
    )?)?;
    command.rationale = text("Different controlled decision for the same immutable operation")?;
    let before = retained_state(&fixture.store)?;
    check!(matches!(
        fixture
            .store
            .prepare_evidence_subject_attachment_v1(command, Preparation::ReplayOnly),
        Err(StoreError::EvidenceSubjectAttachment(
            AttachmentError::OperationConflict
        ))
    ));
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}

#[test]
fn unrelated_attachment_commit_invalidates_a_prepared_inventory_without_changing_source_generations(
) -> TestResult {
    let fixture = Fixture::new()?;
    let first_context = Context::synthetic("inventory-first")?;
    let second_context = Context::synthetic("inventory-second")?;
    let first = fixture.inspect()?;
    let second = fixture.inspect()?;
    let first_command = first_context.command("inventory-first", &first)?;
    let second_command = second_context.command("inventory-second", &second)?;
    let first = prepare(&fixture.store, first, first_command, &first_context)?;
    let second = prepare(&fixture.store, second, second_command, &second_context)?;
    let evidence = fixture.store.evidence_generation();
    let derived = fixture.store.derived_generation();
    written(apply(&fixture.store, first)?)?;
    check!(eq; fixture.store.evidence_generation(), evidence);
    check!(eq; fixture.store.derived_generation(), derived);
    let before = retained_state(&fixture.store)?;
    let approval = second.approval_bytes().to_vec();
    let digest = second.approval_digest().clone();
    check!(matches!(
        fixture
            .store
            .apply_evidence_subject_attachment_v1(second, &approval, &digest),
        Err(StoreError::EvidenceSubjectAttachment(
            AttachmentError::InventoryMoved
        ))
    ));
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}

#[test]
fn changed_declared_head_and_input_generations_cannot_substitute_for_inspected_inputs() -> TestResult
{
    let fixture = Fixture::new()?;
    let context = Context::synthetic("declared-fence")?;
    let before = retained_state(&fixture.store)?;
    for field in ["head", "evidence", "derived"] {
        let inspection = fixture.inspect()?;
        let mut command = context.command("declared-fence", &inspection)?;
        match field {
            "head" => {
                command.expected_public_head.sequence = command
                    .expected_public_head
                    .sequence
                    .checked_add(1)
                    .ok_or("head overflow")?
            }
            "evidence" => {
                command.expected_evidence_generation = command
                    .expected_evidence_generation
                    .checked_add(1)
                    .ok_or("evidence overflow")?
            }
            "derived" => {
                command.expected_derived_generation = command
                    .expected_derived_generation
                    .checked_add(1)
                    .ok_or("derived overflow")?
            }
            _ => return Err("unknown declared fence fixture".into()),
        }
        check!(matches!(
            fixture.store.prepare_evidence_subject_attachment_v1(
                command,
                Preparation::New {
                    inspection,
                    context: &context.borrowed()
                }
            ),
            Err(StoreError::EvidenceSubjectAttachment(
                AttachmentError::InputMoved
            ))
        ));
        check!(eq; retained_state(&fixture.store)?, before);
    }
    Ok(())
}

#[test]
fn appended_source_evidence_invalidates_preparation_without_mutating_any_retained_keyspace(
) -> TestResult {
    let fixture = Fixture::new()?;
    let context = Context::synthetic("source-fence")?;
    let inspection = fixture.inspect()?;
    let command = context.command("source-fence", &inspection)?;
    let prepared = prepare(&fixture.store, inspection, command, &context)?;
    let expected_evidence = fixture.store.evidence_generation();
    let expected_derived = fixture.store.derived_generation();
    let added = crate::public_identity_tests::support::subject("attachment-later-source", "93103")?;
    fixture.store.append(Table::Athletes, &added)?;
    let actual_evidence = fixture.store.evidence_generation();
    let actual_derived = fixture.store.derived_generation();
    check!(ne; actual_evidence, expected_evidence);
    let before = retained_state(&fixture.store)?;
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    match fixture
        .store
        .apply_evidence_subject_attachment_v1(prepared, &approval, &digest)
    {
        Err(StoreError::RevisionMoved {
            operation,
            expected_evidence: old_evidence,
            actual_evidence: new_evidence,
            expected_derived: old_derived,
            actual_derived: new_derived,
        }) => {
            check!(eq; operation, "source-fence");
            check!(eq; (old_evidence, new_evidence), (expected_evidence, actual_evidence));
            check!(eq; (old_derived, new_derived), (expected_derived, actual_derived));
        }
        other => return Err(format!("expected exact source revision fence, got {other:?}").into()),
    }
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}

#[test]
fn a_held_structural_read_poisoned_by_step_exhaustion_never_resets_the_original_grant() -> TestResult
{
    resource_cases::a_held_structural_read_poisoned_by_step_exhaustion_never_resets_the_original_grant()
}

#[test]
fn a_held_structural_read_poisoned_by_byte_exhaustion_preserves_original_charge_and_store(
) -> TestResult {
    resource_cases::a_held_structural_read_poisoned_by_byte_exhaustion_preserves_original_charge_and_store()
}

mod resource_cases;
