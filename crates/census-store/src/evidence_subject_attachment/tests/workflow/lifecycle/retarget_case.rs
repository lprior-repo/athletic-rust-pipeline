use super::*;

pub(super) fn revoked_attachment_replay_never_reactivates_and_different_target_starts_a_fresh_first_chain(
) -> TestResult {
    let fixture = Fixture::new()?;
    let second = fixture.second_birth()?;
    let context = Context::synthetic("retarget")?;
    let inspection = fixture.inspect()?;
    let original_command = context.command("retarget-first", &inspection)?;
    let first_receipt = written(apply(
        &fixture.store,
        prepare(
            &fixture.store,
            inspection,
            original_command.clone(),
            &context,
        )?,
    )?)?;
    let original = first_record(&fixture.store)?;
    let overlapping = inspect_first(&fixture.store, second.public_id())?;
    let command = next_command(&original, "retarget-overlap", &overlapping)?;
    let before_overlap = retained_state(&fixture.store)?;
    check!(matches!(
        fixture.store.prepare_evidence_subject_attachment_v1(
            command,
            Preparation::New {
                inspection: overlapping,
                context: &[]
            }
        ),
        Err(StoreError::EvidenceSubjectAttachment(
            AttachmentError::Domain(EvidenceSubjectAttachmentError::Conflict(
                "overlapping active subject"
            ))
        ))
    ));
    check!(eq; retained_state(&fixture.store)?, before_overlap);
    let revocation = fixture.store.inspect_evidence_subject_attachment_v1(
        AttachmentInspectionRequestV1::Revoke {
            prior: original.reference(),
        },
    )?;
    let revoke = next_command(&original, "retarget-revoke", &revocation)?;
    let prepared = fixture.store.prepare_evidence_subject_attachment_v1(
        revoke,
        Preparation::New {
            inspection: revocation,
            context: &[],
        },
    )?;
    written(apply(&fixture.store, prepared)?)?;
    let after_revoke = retained_state(&fixture.store)?;
    check!(eq; replay(&fixture.store, original_command)?, Application::Repeated(first_receipt));
    check!(eq; retained_state(&fixture.store)?, after_revoke);
    let revoked_view = fixture.store.snapshot();
    let mut revoked_grant = AttachmentReplayGrantV1::new();
    let revoked =
        revoked_view.read_evidence_subject_attachment_references_v1(&mut revoked_grant)?;
    check!(eq; revoked.records().len(), 2);
    check!(eq; active_operations(&revoked)?, Vec::<String>::new());
    let old_records = revoked.records().to_vec();
    let inspection = inspect_first(&fixture.store, second.public_id())?;
    let command = next_command(&original, "retarget-new-first", &inspection)?;
    check!(eq; command.action, AttachmentActionV1::Attach { prior: AttachmentPredecessorV1::First });
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
    check!(eq; read.records().len(), 3);
    check!(eq; read.records().get(..2), Some(old_records.as_slice()));
    check!(eq; read.records().last().ok_or("retarget missing")?.command, command);
    check!(eq; active_operations(&read)?, vec!["retarget-new-first".to_owned()]);
    check!(eq; active_operations(&revoked)?, Vec::<String>::new());
    Ok(())
}
