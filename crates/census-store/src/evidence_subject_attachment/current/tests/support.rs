use super::*;

pub(super) struct Committed {
    pub fixture: Fixture,
    pub context: Context,
    pub record: ReviewedReferenceRecordV1,
    pub receipt: Receipt,
    pub group_bytes: usize,
}

pub(super) fn committed(label: &str) -> TestResult<Committed> {
    let fixture = Fixture::new()?;
    let context = Context::synthetic(label)?;
    let inspection = fixture.inspect()?;
    let group_bytes = usize::try_from(
        inspection
            .issued_objects()
            .next()
            .ok_or("original target group absent")?
            .bytes,
    )?;
    let command = context.command(label, &inspection)?;
    let receipt = written(apply(
        &fixture.store,
        prepare(&fixture.store, inspection, command, &context)?,
    )?)?;
    let record = first_record(&fixture.store)?;
    Ok(Committed {
        fixture,
        context,
        record,
        receipt,
        group_bytes,
    })
}

pub(super) fn poisoned(grant: &mut AttachmentReplayGrantV1) -> TestResult {
    check!(matches!(
        grant.charge_steps(0),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    check!(matches!(
        grant.charge_bytes(0),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}

pub(super) fn replacement_digest() -> TestResult<AttachmentDigestV1> {
    Ok(AttachmentDigestV1::parse(&"0".repeat(64))?)
}

pub(super) fn binding_bytes(case: &Committed) -> TestResult<usize> {
    let binding = AttachmentReceiptBindingV1 {
        operation: case.record.command.operation.clone(),
        record_digest: case.record.record_digest.clone(),
    };
    serde_json::to_vec(&case.record)?
        .len()
        .checked_add(serde_json::to_vec(&binding)?.len())
        .ok_or_else(|| "current projection byte sum overflow".into())
}
