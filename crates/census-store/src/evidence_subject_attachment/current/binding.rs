use super::*;
use census_domain::evidence_subject_attachment::ReviewedReferenceRecordV1;

pub(super) fn bind<'a, 's>(
    owner: &'a CurrentEvidenceSubjectAttachmentReferencesV1<'_, 's>,
    expected: &RetainedDocumentSubjectReferenceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Option<CurrentEvidenceSubjectAttachmentV1<'a, 's>>> {
    let Some((record, subject)) = select(owner, expected, grant)? else {
        return Ok(None);
    };
    let index = owner
        .ledger
        .find_operation(record.command.operation.as_str(), grant)?
        .ok_or(Error::Corrupt("active attachment record missing"))?;
    let receipt = owner
        .ledger
        .receipts
        .get(index)
        .ok_or(Error::Corrupt("active attachment receipt missing"))?;
    let target = super::super::target::current(
        owner.accepted.view(),
        &record.command.target.public_id,
        grant,
    )?;
    target.require_command_target(&record.command.target, grant)?;
    grant.charge_steps(1).map_err(Error::Domain)?;
    Ok(Some(CurrentEvidenceSubjectAttachmentV1 {
        view: owner.view,
        ledger: &owner.ledger,
        accepted: owner.accepted.view(),
        record,
        subject,
        receipt,
    }))
}

fn select<'a>(
    owner: &'a CurrentEvidenceSubjectAttachmentReferencesV1<'_, '_>,
    expected: &RetainedDocumentSubjectReferenceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<
    Option<(
        &'a ReviewedReferenceRecordV1,
        &'a RetainedDocumentSubjectReferenceV1,
    )>,
> {
    let projection = owner
        .ledger
        .projection(owner.accepted.view().store_identity(), grant)?;
    grant
        .charge_steps(owner.ledger.records.len())
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
