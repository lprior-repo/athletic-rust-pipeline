use super::*;
pub(super) use crate::evidence_subject_attachment::material::Material;
use crate::evidence_subject_attachment::ReviewedAttachmentSourceV1;
use crate::read::RetainedJournalReadSource;
use crate::{AttachmentContextObjectV1, StoreSnapshot};
use census_domain::evidence_subject_attachment::{
    AttachmentObjectReferenceV1, AttachmentSubjectRoleV1, ReviewedReferenceRecordV1,
};

mod capture_context;
pub(super) mod checked;
mod documentary;
mod person;
mod statement;

pub(super) fn introduce(
    source: &ResultCalendarSourceClaimV1,
    context: &[AttachmentContextObjectV1<'_>],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<Material>> {
    if context.len() != source.evidence.len() || context.len() > MAX_RESULT_CALENDAR_OBJECTS {
        return Err(Error::SourceMismatch("context inventory").into());
    }
    grant
        .charge_bytes(
            context
                .len()
                .checked_mul(std::mem::size_of::<Material>())
                .ok_or(Error::BudgetExceeded)?,
        )
        .map_err(grant_error)?;
    let mut materials = Vec::new();
    materials
        .try_reserve_exact(context.len())
        .map_err(|_| Error::BudgetExceeded)?;
    for input in context {
        materials.push(Material::introduce(input, grant)?);
    }
    validate_inventory(source, &materials, grant)?;
    Ok(materials)
}

pub(super) fn require_current(
    view: &StoreSnapshot<'_>,
    source: &ResultCalendarSourceClaimV1,
    materials: &[Material],
    current: &crate::CurrentEvidenceSubjectAttachmentReferencesV1<'_, '_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    if !std::ptr::eq(view, current.snapshot())
        || view.sequence() != current.snapshot_sequence()
        || current.inventory() != &source.attachment_inventory
    {
        return Err(Error::InputMoved.into());
    }
    let originals = current.reviewed_source_v1(grant)?;
    require_historical(view, source, materials, &originals, grant)
}

pub(super) fn require_historical(
    view: &impl RetainedJournalReadSource,
    source: &ResultCalendarSourceClaimV1,
    materials: &[Material],
    originals: &ReviewedAttachmentSourceV1<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    validate_inventory(source, materials, grant)?;
    let records = originals.records(grant)?;
    let (result_record, documentary_record) = person::originals(source, records, grant)?;
    let subject = result_record
        .command
        .subjects
        .as_slice()
        .iter()
        .find(|subject| {
            subject.role == AttachmentSubjectRoleV1::ResultRow
                && subject.locator == source.result.row_locator
                && subject.interpretation == source.result.interpretation.digest
                && subject.body == source.result.body.digest
        })
        .ok_or(Error::SourceMismatch("original result subject"))?;
    for material in materials {
        if material.reference() != &source.result.body {
            material.verify(view, grant)?;
        }
    }
    let parser = object(materials, &source.parser_contract)?.inline_bytes(grant)?;
    if parser != RESULT_CALENDAR_PARSER_CONTRACT_V1 {
        return Err(Error::SourceMismatch("calendar parser contract").into());
    }
    let result = root(
        materials,
        &source.result.result_root,
        crate::RetainedCaptureRootSchemaV1::HtmlResultCaptureV1,
        grant,
    )?;
    if result.payload.digest != subject.capture {
        return Err(Error::SourceMismatch("original result capture").into());
    }
    crate::evidence_subject_attachment::external::with_checked(
        view,
        &result,
        grant,
        |_, checked, grant| {
            let crate::evidence_subject_attachment::external::CheckedRoot::Html(capture) = checked
            else {
                return Err(Error::SourceMismatch("result capture schema").into());
            };
            statement::require(source, materials, subject, &result, &capture, grant)
        },
    )?;
    documentary::require(
        view,
        source,
        materials,
        (originals, documentary_record),
        grant,
    )
}

pub(super) fn validate_inventory(
    source: &ResultCalendarSourceClaimV1,
    materials: &[Material],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let evidence = source.evidence.as_slice();
    if evidence.is_empty()
        || evidence.len() > MAX_RESULT_CALENDAR_OBJECTS
        || materials.len() != evidence.len()
    {
        return Err(Error::SourceMismatch("context count").into());
    }
    grant
        .charge_steps(
            evidence
                .len()
                .checked_mul(evidence.len())
                .ok_or(Error::BudgetExceeded)?,
        )
        .map_err(grant_error)?;
    if evidence.windows(2).any(|pair| match pair {
        [left, right] => left.digest >= right.digest,
        _ => true,
    }) {
        return Err(Error::SourceMismatch("context order or duplicates").into());
    }
    for reference in evidence {
        if materials
            .iter()
            .filter(|material| material.reference() == reference)
            .count()
            != 1
        {
            return Err(Error::SourceMismatch("missing or duplicate context").into());
        }
    }
    for reference in [
        &source.result.result_root,
        &source.result.body,
        &source.result.interpretation,
        &source.calendar.documentary_root,
        &source.parser_contract,
    ] {
        object(materials, reference)?;
    }
    Ok(())
}

pub(super) fn object<'a>(
    materials: &'a [Material],
    reference: &AttachmentObjectReferenceV1,
) -> StoreResult<&'a Material> {
    materials
        .iter()
        .find(|material| material.reference() == reference)
        .ok_or_else(|| Error::SourceMismatch("undeclared context object").into())
}

pub(super) fn root(
    materials: &[Material],
    reference: &AttachmentObjectReferenceV1,
    schema: crate::RetainedCaptureRootSchemaV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<crate::RetainedCaptureRootV1> {
    let bytes = object(materials, reference)?.inline_bytes(grant)?;
    let root: crate::RetainedCaptureRootV1 = codec::decode(&bytes, grant)?;
    if root.schema != schema {
        return Err(Error::SourceMismatch("capture root schema").into());
    }
    Ok(root)
}

pub(super) fn require_accepted_head(
    head: &census_domain::evidence_subject_attachment::AttachmentPublicHeadV1,
    accepted: &crate::AcceptedPublicIdentitySnapshot,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    grant
        .charge_steps(
            accepted
                .history_records()
                .len()
                .checked_add(2)
                .ok_or(Error::BudgetExceeded)?,
        )
        .map_err(grant_error)?;
    let current =
        accepted.head().sequence == head.sequence && accepted.head().digest == head.digest.as_str();
    let historical = accepted
        .history_records()
        .iter()
        .any(|record| record.sequence == head.sequence && record.digest == head.digest.as_str());
    let genesis = accepted.history_records().first().is_some_and(|record| {
        record.previous.sequence == head.sequence && record.previous.digest == head.digest.as_str()
    });
    if !current && !historical && !genesis {
        return Err(Error::SourceMismatch("original accepted identity head").into());
    }
    Ok(())
}
