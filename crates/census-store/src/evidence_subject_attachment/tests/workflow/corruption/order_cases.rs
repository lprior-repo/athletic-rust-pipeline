use super::*;

pub(super) fn raw_out_of_order_introductions_refuse_before_sorted_lookup_can_hide_them(
) -> TestResult {
    let (fixture, _context, _record) = committed("raw-introduction-order")?;
    let raw = fixture
        .store
        .journal
        .get(Store::journal_key("evidence_subject", RECORD_KEY))?
        .ok_or("first entry absent")?;
    let mut envelope: CorruptedEnvelope = serde_json::from_slice(&raw)?;
    let original_record = envelope.payload.record.get().to_owned();
    check!(envelope.payload.introduced.len() >= 2);
    envelope.payload.introduced.swap(0, 1);
    replace_typed_entry(&fixture.store, RECORD_KEY, &envelope, &original_record)?;
    let before = retained_state(&fixture.store)?;
    let mut grant = AttachmentReplayGrantV1::new();
    check!(matches!(
        fixture
            .store
            .snapshot()
            .read_evidence_subject_attachment_references_v1(&mut grant),
        Err(StoreError::EvidenceSubjectAttachment(
            AttachmentError::Corrupt("duplicate or unordered context introduction")
        ))
    ));
    poisoned_without_further_reads(&fixture.store, &mut grant)?;
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}

pub(super) fn raw_duplicate_across_prior_introduction_sets_refuses_with_both_historical_records_unchanged(
) -> TestResult {
    let (fixture, _context, original) = committed("raw-prior-duplicate")?;
    let inspection = fixture.store.inspect_evidence_subject_attachment_v1(
        AttachmentInspectionRequestV1::Attach {
            target: original.command.target.public_id.clone(),
            prior: AttachmentPredecessorV1::After(original.reference()),
        },
    )?;
    let command = next_command(&original, "raw-prior-duplicate-after", &inspection)?;
    let prepared = fixture.store.prepare_evidence_subject_attachment_v1(
        command,
        Preparation::New {
            inspection,
            context: &[],
        },
    )?;
    written(apply(&fixture.store, prepared)?)?;
    let first_raw = fixture
        .store
        .journal
        .get(Store::journal_key("evidence_subject", RECORD_KEY))?
        .ok_or("first entry absent")?;
    let mut first: CorruptedEnvelope = serde_json::from_slice(&first_raw)?;
    let duplicate = first
        .payload
        .introduced
        .pop()
        .ok_or("first introduction absent")?;
    let second_key = "attachment/v1/record/00000000000000000002";
    let second_raw = fixture
        .store
        .journal
        .get(Store::journal_key("evidence_subject", second_key))?
        .ok_or("second entry absent")?;
    let mut second: CorruptedEnvelope = serde_json::from_slice(&second_raw)?;
    let second_record = second.payload.record.get().to_owned();
    check!(!second
        .payload
        .introduced
        .iter()
        .any(|item| item.reference == duplicate.reference));
    second.payload.introduced.push(duplicate);
    second
        .payload
        .introduced
        .sort_by(|left, right| left.reference.cmp(&right.reference));
    replace_typed_entry(&fixture.store, second_key, &second, &second_record)?;
    check!(eq; fixture.store.journal.get(Store::journal_key("evidence_subject", RECORD_KEY))?
        .ok_or("first entry disappeared")?.as_ref(), first_raw.as_ref());
    let before = retained_state(&fixture.store)?;
    let mut grant = AttachmentReplayGrantV1::new();
    check!(matches!(
        fixture
            .store
            .snapshot()
            .read_evidence_subject_attachment_references_v1(&mut grant),
        Err(StoreError::EvidenceSubjectAttachment(
            AttachmentError::Corrupt("duplicate or unordered context introduction")
        ))
    ));
    poisoned_without_further_reads(&fixture.store, &mut grant)?;
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}
