use super::*;
use crate::public_identity::paged_generation::{
    decode_attachment_capture_binding, require_attachment_retained_inputs,
};

pub(super) fn verify(
    ledger: &Ledger<'_>,
    record: &ReviewedReferenceRecordV1,
    inputs: &wire::EntryInputs,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    for expected in [&inputs.capture, &inputs.token] {
        declared(record, expected, grant)?;
    }
    let capture = ledger
        .object(&inputs.capture.digest, grant)?
        .inline_bytes(grant)?;
    grant.charge_steps(3).map_err(Error::Domain)?;
    let capture = decode_attachment_capture_binding(&capture)?;
    let token = ledger
        .object(&inputs.token.digest, grant)?
        .inline_bytes(grant)?;
    grant.charge_steps(5).map_err(Error::Domain)?;
    require_attachment_retained_inputs(
        &token,
        &capture,
        &record.command.store,
        record.command.expected_evidence_generation,
        record.command.expected_derived_generation,
    )
}

fn declared(
    record: &ReviewedReferenceRecordV1,
    expected: &AttachmentObjectReferenceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    if super::declared(
        record.command.reviewed_context.objects.as_slice(),
        expected,
        grant,
    )? {
        return Ok(());
    }
    Err(Error::Corrupt("entry input role absent from complete context").into())
}
