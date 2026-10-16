use super::super::fixture::*;
use crate::evidence_subject_attachment::{
    EvidenceSubjectAttachmentPreparationV1 as Preparation,
    EvidenceSubjectAttachmentStoreError as AttachmentError,
};
use crate::public_identity_tests::support::{retained_state, TestResult};
use crate::{Application, StoreError};
use census_domain::evidence_subject_attachment::*;

#[test]
fn changed_inventory_before_preparation_allows_only_exact_existing_replay_not_a_new_append(
) -> TestResult {
    let fixture = Fixture::new()?;
    let pending_context = Context::synthetic("owner-reuse-pending")?;
    let pending = fixture.inspect()?;
    let command = pending_context.command("owner-reuse-pending", &pending)?;
    let winner_context = Context::synthetic("owner-reuse-winner")?;
    let winner = fixture.inspect()?;
    let winner_command = winner_context.command("owner-reuse-winner", &winner)?;
    let receipt = written(apply(
        &fixture.store,
        prepare(
            &fixture.store,
            winner,
            winner_command.clone(),
            &winner_context,
        )?,
    )?)?;
    let before = retained_state(&fixture.store)?;
    check!(matches!(
        fixture.store.prepare_evidence_subject_attachment_v1(
            command,
            Preparation::New {
                inspection: pending,
                context: &pending_context.borrowed()
            },
        ),
        Err(StoreError::EvidenceSubjectAttachment(
            AttachmentError::InventoryMoved
        ))
    ));
    check!(eq; retained_state(&fixture.store)?, before);
    check!(eq; first_record(&fixture.store)?.command, winner_command);
    check!(eq; fixture.store.receipt(&receipt.operation)?, Some(receipt));
    Ok(())
}

#[test]
fn changed_inventory_during_prepared_command_race_preserves_full_command_conflict() -> TestResult {
    let fixture = Fixture::new()?;
    let context = Context::synthetic("owner-reuse-operation-race")?;
    let winner = fixture.inspect()?;
    let loser = fixture.inspect()?;
    let winner_command = context.command("owner-reuse-operation-race", &winner)?;
    let mut loser_command = context.command("owner-reuse-operation-race", &loser)?;
    loser_command.rationale =
        text("Explicit different controlled review; command conflict must survive refresh")?;
    check!(ne; winner_command, loser_command);
    let winner = prepare(&fixture.store, winner, winner_command.clone(), &context)?;
    let loser = prepare(&fixture.store, loser, loser_command, &context)?;
    let receipt = written(apply(&fixture.store, winner)?)?;
    let before = retained_state(&fixture.store)?;
    let approval = loser.approval_bytes().to_vec();
    let digest = loser.approval_digest().clone();
    check!(matches!(
        fixture
            .store
            .apply_evidence_subject_attachment_v1(loser, &approval, &digest),
        Err(StoreError::EvidenceSubjectAttachment(
            AttachmentError::OperationConflict
        ))
    ));
    check!(eq; retained_state(&fixture.store)?, before);
    check!(eq; first_record(&fixture.store)?.command, winner_command);
    check!(eq; fixture.store.receipt(&receipt.operation)?, Some(receipt));
    Ok(())
}

#[test]
fn prepared_exact_replay_survives_unrelated_append_through_fresh_canonical_replay() -> TestResult {
    let fixture = Fixture::new()?;
    let first_context = Context::synthetic("owner-reuse-replay-first")?;
    let first = fixture.inspect()?;
    let first_command = first_context.command("owner-reuse-replay-first", &first)?;
    let first_receipt = written(apply(
        &fixture.store,
        prepare(&fixture.store, first, first_command.clone(), &first_context)?,
    )?)?;
    let first_record = first_record(&fixture.store)?;
    let replay = fixture
        .store
        .prepare_evidence_subject_attachment_v1(first_command, Preparation::ReplayOnly)?;
    check!(replay.is_replay());
    let second_context = Context::synthetic("owner-reuse-replay-second")?;
    let second = fixture.inspect()?;
    let second_command = second_context.command("owner-reuse-replay-second", &second)?;
    let second_receipt = written(apply(
        &fixture.store,
        prepare(
            &fixture.store,
            second,
            second_command.clone(),
            &second_context,
        )?,
    )?)?;
    let before = retained_state(&fixture.store)?;
    check!(eq; apply(&fixture.store, replay)?, Application::Repeated(first_receipt.clone()));
    check!(eq; retained_state(&fixture.store)?, before);
    let view = fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let read = view.read_evidence_subject_attachment_references_v1(&mut grant)?;
    check!(eq; read.records().len(), 2);
    check!(eq; read.records().first(), Some(&first_record));
    check!(eq; read.records().last().ok_or("second record absent")?.command, second_command);
    check!(eq; read.receipts(), &[first_receipt, second_receipt]);
    Ok(())
}

#[test]
fn changed_nonempty_inspection_replays_exact_original_before_unused_new_context_fences(
) -> TestResult {
    let case = super::support::committed("owner-reuse-original-before-new-replay")?;
    let pending = case.fixture.inspect()?;
    let second_context = Context::synthetic("owner-reuse-new-replay-unrelated")?;
    let second = case.fixture.inspect()?;
    let command = second_context.command("owner-reuse-new-replay-unrelated", &second)?;
    let second_receipt = written(apply(
        &case.fixture.store,
        prepare(&case.fixture.store, second, command, &second_context)?,
    )?)?;
    let before = retained_state(&case.fixture.store)?;
    let replay = case.fixture.store.prepare_evidence_subject_attachment_v1(
        case.record.command.clone(),
        Preparation::New {
            inspection: pending,
            context: &[],
        },
    )?;
    check!(replay.is_replay());
    check!(eq; retained_state(&case.fixture.store)?, before);
    check!(eq; apply(&case.fixture.store, replay)?, Application::Repeated(case.receipt.clone()));
    check!(eq; retained_state(&case.fixture.store)?, before);
    let view = case.fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let read = view.read_evidence_subject_attachment_references_v1(&mut grant)?;
    check!(eq; read.records().len(), 2);
    check!(eq; read.records().first(), Some(&case.record));
    check!(eq; read.receipts(), &[case.receipt, second_receipt]);
    Ok(())
}
