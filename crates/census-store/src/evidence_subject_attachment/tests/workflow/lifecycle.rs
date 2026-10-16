use super::fixture::*;
use crate::evidence_subject_attachment::{
    AttachmentContextObjectV1, AttachmentInspectionRequestV1,
    EvidenceSubjectAttachmentPreparationV1 as Preparation,
    EvidenceSubjectAttachmentStoreError as AttachmentError,
};
use crate::public_identity_tests::support::{retained_state, TestResult};
use crate::{Application, Store, StoreError};
use census_domain::evidence_subject_attachment::*;

#[test]
fn source_bound_public_target_roundtrips_actual_inspection_preparation_application_and_reopen(
) -> TestResult {
    let fixture = Fixture::new()?;
    let context = Context::synthetic("roundtrip")?;
    let held = fixture.store.snapshot();
    let before = retained_state(&fixture.store)?;
    let inspection = fixture.inspect()?;
    check!(eq; inspection.target().public_id, *fixture.first.public_id());
    check!(eq; inspection.public_head().sequence, fixture.first.head().sequence);
    check!(eq; inspection.inventory(), &AttachmentInventoryV1::empty()?);
    let issued = inspection.issued_objects().cloned().collect::<Vec<_>>();
    let command = context.command("roundtrip", &inspection)?;
    let prepared = prepare(&fixture.store, inspection, command.clone(), &context)?;
    check!(eq; prepared.command(), &command);
    check!(eq; prepared.approval_bytes(), command.approval_bytes(4 * 1024 * 1024)?.as_slice());
    check!(eq; prepared.approval_digest(), &command.approval_digest()?);
    check!(!prepared.is_replay());
    let receipt = written(apply(&fixture.store, prepared)?)?;
    let mut old_grant = AttachmentReplayGrantV1::new();
    let old = held.read_evidence_subject_attachment_references_v1(&mut old_grant)?;
    check!(eq; old.records(), &[]);
    check!(eq; old.receipts(), &[]);
    check!(eq; old.inventory(), &AttachmentInventoryV1::empty()?);
    let view = fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let read = view.read_evidence_subject_attachment_references_v1(&mut grant)?;
    check!(eq; read.records().len(), 1);
    check!(eq; read.receipts(), std::slice::from_ref(&receipt));
    check!(eq; read.inventory().records, 1);
    let record = read
        .records()
        .first()
        .ok_or("committed attachment missing")?
        .clone();
    check!(eq; record.command, command);
    check!(eq; record.prepared_digest, command.prepared_digest()?);
    check!(eq; record.approval_digest, command.approval_digest()?);
    check!(eq; record.record_digest, record.calculated_record_digest()?);
    check!(eq; receipt.operation, "evidence_subject_attachment_v1:roundtrip");
    check!(eq; receipt.digest, record.record_digest.as_str());
    check!(eq; receipt.at, record.committed_at.date_naive().to_string());
    check!(eq; receipt.appended, 0);
    check!(!receipt.credited);
    let inventory = read.inventory().clone();
    let after = retained_state(&fixture.store)?;
    super::physical::exact_inline_append(&before, &after, &context, &issued, &record, &receipt)?;
    check!(eq; fixture.writer.head(&fixture.store)?, fixture.first.head().clone());
    check!(grant.usage().encoded_bytes() > 0);
    check!(grant.usage().traversal_steps() > 0);
    drop(read);
    drop(view);
    drop(old);
    drop(held);
    let Fixture {
        directory,
        store,
        writer,
        first,
        ..
    } = fixture;
    drop(writer);
    drop(store);
    let (reopened, writer) =
        Store::open_public_identity_qualification_v3(directory.path().join("identity"))?
            .into_parts();
    check!(eq; retained_state(&reopened)?, after);
    check!(eq; writer.head(&reopened)?, first.head().clone());
    let view = reopened.snapshot();
    let mut original_grant = AttachmentReplayGrantV1::new();
    let read = view.read_evidence_subject_attachment_references_v1(&mut original_grant)?;
    check!(eq; read.records(), std::slice::from_ref(&record));
    check!(eq; read.receipts(), std::slice::from_ref(&receipt));
    check!(eq; read.inventory(), &inventory);
    check!(eq; replay(&reopened, command)?, Application::Repeated(receipt));
    check!(eq; retained_state(&reopened)?, after);
    Ok(())
}

#[test]
fn earlier_independent_inspection_replays_the_original_command_after_first_commit() -> TestResult {
    let fixture = Fixture::new()?;
    let context = Context::synthetic("prepared-race")?;
    let first = fixture.inspect()?;
    let second = fixture.inspect()?;
    let command = context.command("prepared-race", &first)?;
    let first = prepare(&fixture.store, first, command.clone(), &context)?;
    let receipt = written(apply(&fixture.store, first)?)?;
    let after_first = retained_state(&fixture.store)?;
    let second = prepare(&fixture.store, second, command.clone(), &context)?;
    check!(second.is_replay());
    check!(eq; apply(&fixture.store, second)?, Application::Repeated(receipt.clone()));
    check!(eq; retained_state(&fixture.store)?, after_first);
    check!(eq; replay(&fixture.store, command)?, Application::Repeated(receipt));
    check!(eq; retained_state(&fixture.store)?, after_first);
    Ok(())
}

#[test]
fn exact_durable_replay_ignores_unused_malformed_alternate_context_after_inspection_is_consumed(
) -> TestResult {
    let fixture = Fixture::new()?;
    let context = Context::synthetic("unused-replay-context")?;
    let first = fixture.inspect()?;
    let alternate = fixture.inspect()?;
    let command = context.command("unused-replay-context", &first)?;
    let receipt = written(apply(
        &fixture.store,
        prepare(&fixture.store, first, command.clone(), &context)?,
    )?)?;
    let object = context.objects.first().ok_or("controlled object missing")?;
    let malformed = [AttachmentContextObjectV1::Inline {
        reference: &object.reference,
        bytes: b"different bytes with the original declared digest",
    }];
    let before = retained_state(&fixture.store)?;
    let repeated = fixture.store.prepare_evidence_subject_attachment_v1(
        command,
        Preparation::New {
            inspection: alternate,
            context: &malformed,
        },
    )?;
    check!(repeated.is_replay());
    check!(eq; apply(&fixture.store, repeated)?, Application::Repeated(receipt));
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}

#[test]
fn original_command_replays_after_later_source_birth_and_retains_old_history() -> TestResult {
    let fixture = Fixture::new()?;
    let context = Context::synthetic("head-movement-replay")?;
    let inspection = fixture.inspect()?;
    let command = context.command("head-movement-replay", &inspection)?;
    let receipt = written(apply(
        &fixture.store,
        prepare(&fixture.store, inspection, command.clone(), &context)?,
    )?)?;
    let original = first_record(&fixture.store)?;
    let later = fixture.second_birth()?;
    check!(ne; later.head(), fixture.first.head());
    let before = retained_state(&fixture.store)?;
    check!(eq; replay(&fixture.store, command)?, Application::Repeated(receipt));
    check!(eq; first_record(&fixture.store)?, original);
    check!(eq; retained_state(&fixture.store)?, before);
    check!(eq; fixture.writer.head(&fixture.store)?, later.head().clone());
    Ok(())
}

#[test]
fn active_replacement_uses_exact_latest_chain_and_keeps_historical_witness_after_head_advance(
) -> TestResult {
    let fixture = Fixture::new()?;
    let context = Context::synthetic("replacement")?;
    let inspection = fixture.inspect()?;
    let command = context.command("replacement-first", &inspection)?;
    written(apply(
        &fixture.store,
        prepare(&fixture.store, inspection, command, &context)?,
    )?)?;
    let original = first_record(&fixture.store)?;
    let later = fixture.second_birth()?;
    let inspection = fixture.store.inspect_evidence_subject_attachment_v1(
        AttachmentInspectionRequestV1::Attach {
            target: fixture.first.public_id().clone(),
            prior: AttachmentPredecessorV1::After(original.reference()),
        },
    )?;
    check!(eq; inspection.accepted_target_witness(), &original.command.reviewed_context.accepted_target_witness);
    check!(eq; inspection.public_head().sequence, later.head().sequence);
    let command = next_command(&original, "replacement-second", &inspection)?;
    let prepared = fixture.store.prepare_evidence_subject_attachment_v1(
        command.clone(),
        Preparation::New {
            inspection,
            context: &[],
        },
    )?;
    written(apply(&fixture.store, prepared)?)?;
    let view = fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let read = view.read_evidence_subject_attachment_references_v1(&mut grant)?;
    check!(eq; read.records().len(), 2);
    check!(eq; read.records().first(), Some(&original));
    check!(eq; read.records().last().ok_or("replacement missing")?.command, command);
    check!(eq; active_operations(&read)?, vec!["replacement-second".to_owned()]);
    let stale = fixture.store.inspect_evidence_subject_attachment_v1(
        AttachmentInspectionRequestV1::Attach {
            target: fixture.first.public_id().clone(),
            prior: AttachmentPredecessorV1::After(original.reference()),
        },
    )?;
    let stale_command = next_command(&original, "replacement-stale", &stale)?;
    let before = retained_state(&fixture.store)?;
    check!(matches!(
        fixture.store.prepare_evidence_subject_attachment_v1(
            stale_command,
            Preparation::New {
                inspection: stale,
                context: &[]
            }
        ),
        Err(StoreError::EvidenceSubjectAttachment(
            AttachmentError::Domain(EvidenceSubjectAttachmentError::Conflict(
                "predecessor is not exact latest"
            ))
        ))
    ));
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}

#[test]
fn revoked_attachment_replay_never_reactivates_and_different_target_starts_a_fresh_first_chain(
) -> TestResult {
    retarget_case::revoked_attachment_replay_never_reactivates_and_different_target_starts_a_fresh_first_chain()
}

mod retarget_case;
