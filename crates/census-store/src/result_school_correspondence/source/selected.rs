use super::super::issuance::ledger::Ledger;
use super::*;

pub(in super::super) fn physical_school(
    ledger: &Ledger,
    digest: &ResultSchoolDigestV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<census_domain::model::CanonicalSchool> {
    let (record, context) = selected(ledger, digest, grant)?;
    let bytes = object(
        context,
        &record.command.source.target.physical_school_inventory,
    )?
    .inline_bytes(grant)?;
    codec::decode(&bytes, grant)
}

pub(in super::super) fn provider_school_root(
    ledger: &Ledger,
    digest: &ResultSchoolDigestV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<crate::RetainedCaptureRootV1> {
    let (record, context) = selected(ledger, digest, grant)?;
    let bytes =
        object(context, &record.command.source.target.provider_school_root)?.inline_bytes(grant)?;
    codec::decode(&bytes, grant)
}

fn selected<'a>(
    ledger: &'a Ledger,
    digest: &ResultSchoolDigestV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<(&'a ResultSchoolReviewedRecordV1, &'a [Material])> {
    grant
        .charge_steps(ledger.records.len())
        .map_err(grant_error)?;
    let index = ledger
        .records
        .iter()
        .position(|record| &record.record_digest == digest)
        .ok_or(Error::SourceMismatch("unknown reviewed record"))?;
    let record = ledger
        .records
        .get(index)
        .ok_or(Error::Corrupt("selected record missing"))?;
    let context = ledger
        .contexts
        .get(index)
        .ok_or(Error::Corrupt("selected context missing"))?;
    Ok((record, context))
}
