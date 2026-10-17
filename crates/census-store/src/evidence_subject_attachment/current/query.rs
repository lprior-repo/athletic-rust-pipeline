use super::super::ledger::VerifiedLedger;
use super::*;
use crate::Receipt;
use census_domain::evidence_subject_attachment::ReviewedReferenceRecordV1;

pub(in super::super) struct BindingParts<'a> {
    pub(in super::super) record: &'a ReviewedReferenceRecordV1,
    pub(in super::super) subject: &'a RetainedDocumentSubjectReferenceV1,
    pub(in super::super) receipt: &'a Receipt,
}

pub(in super::super) fn bind<'a, P>(
    ledger: &'a VerifiedLedger<P>,
    accepted: &AcceptedPublicIdentitySnapshot,
    expected: &RetainedDocumentSubjectReferenceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Option<BindingParts<'a>>> {
    let Some((record, subject)) = select(ledger, accepted, expected, grant)? else {
        return Ok(None);
    };
    let index = ledger
        .find_operation(record.command.operation.as_str(), grant)?
        .ok_or(Error::Corrupt("active attachment record missing"))?;
    let receipt = ledger
        .receipts
        .get(index)
        .ok_or(Error::Corrupt("active attachment receipt missing"))?;
    let target = super::super::target::current(accepted, &record.command.target.public_id, grant)?;
    target.require_command_target(&record.command.target, grant)?;
    grant.charge_steps(1).map_err(Error::Domain)?;
    Ok(Some(BindingParts {
        record,
        subject,
        receipt,
    }))
}

fn select<'a, P>(
    ledger: &'a VerifiedLedger<P>,
    accepted: &AcceptedPublicIdentitySnapshot,
    expected: &RetainedDocumentSubjectReferenceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<
    Option<(
        &'a ReviewedReferenceRecordV1,
        &'a RetainedDocumentSubjectReferenceV1,
    )>,
> {
    let projection = ledger.projection(accepted.store_identity(), grant)?;
    grant
        .charge_steps(ledger.records.len())
        .map_err(Error::Domain)?;
    let mut selected = None;
    for record in projection.active_records() {
        for subject in record.command.subjects.as_slice() {
            grant.charge_steps(1).map_err(Error::Domain)?;
            if subject.subject != expected.subject {
                continue;
            }
            grant.charge_steps(1).map_err(Error::Domain)?;
            if subject != expected {
                return Err(Error::Corrupt("active attachment subject differs").into());
            }
            grant.charge_steps(1).map_err(Error::Domain)?;
            if selected.is_some() {
                return Err(Error::Corrupt("active attachment subject is ambiguous").into());
            }
            selected = Some((record, subject));
        }
    }
    Ok(selected)
}
