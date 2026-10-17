use super::*;

pub(super) const RECORD_KEY: &str = "attachment/v1/record/00000000000000000001";

pub(super) struct Committed {
    pub fixture: Fixture,
    pub context: Context,
    pub receipt: Receipt,
    pub record: ReviewedReferenceRecordV1,
}

pub(super) fn append(
    fixture: &Fixture,
    label: &str,
) -> TestResult<(Context, Receipt, ReviewedReferenceRecordV1)> {
    let context = Context::synthetic(label)?;
    let inspection = fixture.inspect()?;
    let command = context.command(label, &inspection)?;
    let receipt = written(apply(
        &fixture.store,
        prepare(&fixture.store, inspection, command, &context)?,
    )?)?;
    let view = fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let read = view.read_evidence_subject_attachment_references_v1(&mut grant)?;
    let record = read
        .records()
        .last()
        .ok_or("appended replay fixture record absent")?
        .clone();
    Ok((context, receipt, record))
}

pub(super) fn committed(label: &str) -> TestResult<Committed> {
    let fixture = Fixture::new()?;
    let (context, receipt, record) = append(&fixture, label)?;
    Ok(Committed {
        fixture,
        context,
        receipt,
        record,
    })
}

pub(super) fn unchanged_replay(
    store: &Store,
    record: &ReviewedReferenceRecordV1,
    receipt: &Receipt,
) -> TestResult {
    let before = retained_state(store)?;
    let prepared = store
        .prepare_evidence_subject_attachment_v1(record.command.clone(), Preparation::ReplayOnly)?;
    check!(prepared.is_replay());
    check!(eq; prepared.command(), &record.command);
    check!(eq; prepared.approval_digest(), &record.approval_digest);
    check!(eq; prepared.approval_bytes(), record.command.approval_bytes(4 * 1024 * 1024)?.as_slice());
    check!(eq; apply(store, prepared)?, Application::Repeated(receipt.clone()));
    check!(eq; retained_state(store)?, before);
    Ok(())
}

pub(super) fn refuse_corrupt(
    store: &Store,
    prepared: crate::PreparedEvidenceSubjectAttachmentV1<'_>,
    expected: &'static str,
) -> TestResult {
    let before = retained_state(store)?;
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    check!(
        matches!(store.apply_evidence_subject_attachment_v1(prepared, &approval, &digest),
        Err(StoreError::EvidenceSubjectAttachment(AttachmentError::Corrupt(actual))) if actual == expected)
    );
    check!(eq; retained_state(store)?, before);
    Ok(())
}

pub(super) fn replace_ascii(raw: &[u8], needle: &[u8]) -> TestResult<Vec<u8>> {
    check!(!needle.is_empty());
    let positions = raw
        .windows(needle.len())
        .enumerate()
        .filter_map(|(position, value)| (value == needle).then_some(position))
        .collect::<Vec<_>>();
    let [position] = positions.as_slice() else {
        return Err("corruption marker is not unique".into());
    };
    let mut changed = raw.to_vec();
    let byte = changed
        .get_mut(*position)
        .ok_or("corruption offset absent")?;
    *byte = if *byte == b'0' { b'1' } else { b'0' };
    check!(eq; changed.len(), raw.len());
    check!(ne; changed.as_slice(), raw);
    Ok(changed)
}
