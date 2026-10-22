use super::*;
use crate::evidence_subject_attachment::external::CheckedRoot;
use crate::{
    RetainedCaptureLeafKindV1 as Kind, RetainedCaptureLeafV1, RetainedCaptureRootV1,
    RetainedCaptureSlotV1 as Slot,
};
use census_domain::evidence_subject_attachment::{
    AttachmentDigestV1, RetainedDocumentSubjectReferenceV1,
};
use census_domain::roster_affiliation_authority::SourceRowLocator;

mod preimage;
mod roncalli;
use preimage::{inline, interpretation, locator_range};

pub(super) fn require(
    view: &impl RetainedJournalReadSource,
    source: &ResultCalendarSourceClaimV1,
    materials: &[Material],
    authority: (&ReviewedAttachmentSourceV1<'_>, &ReviewedReferenceRecordV1),
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let parser = super::selected_parser(source, materials, grant)?;
    if !parser.supports_scope(source.sport, source.category) {
        return Err(Error::SourceMismatch("unsupported documentary parser scope").into());
    }
    let root = super::root(
        materials,
        &source.calendar.documentary_root,
        crate::RetainedCaptureRootSchemaV1::ParticipationPacketV1,
        grant,
    )?;
    leaf(
        materials,
        &source.calendar.capture,
        &root,
        Slot::ParticipationSchedule,
        Kind::OriginalBody,
    )?;
    crate::evidence_subject_attachment::external::with_checked(
        view,
        &root,
        grant,
        |_, checked, grant| packet(source, materials, &root, authority, checked, grant),
    )
}

fn packet(
    source: &ResultCalendarSourceClaimV1,
    materials: &[Material],
    root: &RetainedCaptureRootV1,
    authority: (&ReviewedAttachmentSourceV1<'_>, &ReviewedReferenceRecordV1),
    checked: CheckedRoot<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let CheckedRoot::Participation {
        status,
        schedule,
        result_row,
        roncalli_season_event,
    } = checked
    else {
        return Err(Error::SourceMismatch("documentary packet schema").into());
    };
    let subject = original_subject(materials, root, authority, result_row, grant)?;
    if subject.namespace.as_str() == "milesplit_result_document_v1" {
        status.require_reference(&retained(materials, &subject.capture)?.object, grant)?;
    }
    let parser = super::selected_parser(source, materials, grant)?;
    let interpretation_name = parser
        .documentary_interpretation(
            subject.namespace.as_str(),
            result_row.is_some(),
            roncalli_season_event.is_some(),
        )
        .ok_or(Error::SourceMismatch(
            "unknown documentary namespace or row",
        ))?;
    if status.body_sha256 != subject.body.as_str()
        || schedule.body_sha256 != source.calendar.capture.as_str()
    {
        return Err(Error::SourceMismatch("documentary original capture binding").into());
    }
    if let Some(selection) = roncalli_season_event {
        return roncalli::require(
            source,
            materials,
            subject,
            &selection,
            (&status, &schedule),
            grant,
        );
    }
    interpret_documents(source, materials, subject, interpretation_name, grant)
}

fn interpret_documents(
    source: &ResultCalendarSourceClaimV1,
    materials: &[Material],
    subject: &RetainedDocumentSubjectReferenceV1,
    interpretation_name: &str,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    interpretation(
        materials,
        &subject.body,
        &subject.locator,
        &subject.interpretation,
        interpretation_name,
        grant,
    )?;
    interpretation(
        materials,
        &source.calendar.capture,
        &source.calendar.locator,
        &source.calendar.interpretation,
        interpretation_name,
        grant,
    )
}

fn original_subject<'a>(
    materials: &[Material],
    root: &RetainedCaptureRootV1,
    authority: (
        &ReviewedAttachmentSourceV1<'_>,
        &'a ReviewedReferenceRecordV1,
    ),
    result_row: Option<SourceRowLocator>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<&'a RetainedDocumentSubjectReferenceV1> {
    let (originals, record) = authority;
    let subjects = record.command.subjects.as_slice();
    grant.charge_steps(subjects.len()).map_err(grant_error)?;
    let mut selected = None;
    for subject in subjects {
        if subject.role != AttachmentSubjectRoleV1::AthleteStatus {
            continue;
        }
        let original = originals
            .bind(record, subject, grant)?
            .ok_or(Error::SourceMismatch("original documentary owner"))?;
        let capture = original.capture(grant)?;
        let body = original.body(grant)?;
        if subject.namespace.as_str() == "milesplit_result_document_v1" {
            original.root_anchor(root, grant)?;
            if !within_row(body, &original.locator(grant)?, result_row)? {
                continue;
            }
            let retained_body = retained(materials, &subject.body)?;
            if capture.version != 1
                || capture.kind != Kind::CaptureJson
                || retained_body.version != 1
                || retained_body.kind != Kind::OriginalBody
                || retained(materials, &subject.capture)? != capture
                || &retained_body.object != body
            {
                return Err(Error::SourceMismatch("original documentary material binding").into());
            }
        } else {
            if capture.root != *root {
                continue;
            }
            legacy_binding(materials, root, subject, capture, body)?;
        }
        if selected.replace(subject).is_some() {
            return Err(Error::SourceMismatch("ambiguous original documentary subject").into());
        }
    }
    selected.ok_or_else(|| Error::SourceMismatch("original documentary subject").into())
}

fn legacy_binding(
    materials: &[Material],
    root: &RetainedCaptureRootV1,
    subject: &RetainedDocumentSubjectReferenceV1,
    capture: &RetainedCaptureLeafV1,
    body: &AttachmentObjectReferenceV1,
) -> StoreResult<()> {
    let retained_capture = leaf(
        materials,
        &subject.capture,
        root,
        Slot::ParticipationStatus,
        Kind::CaptureJson,
    )?;
    let retained_body = leaf(
        materials,
        &subject.body,
        root,
        Slot::ParticipationStatus,
        Kind::OriginalBody,
    )?;
    if retained_capture != capture || &retained_body.object != body {
        return Err(Error::SourceMismatch("original documentary material binding").into());
    }
    Ok(())
}

fn leaf<'a>(
    materials: &'a [Material],
    digest: &AttachmentDigestV1,
    root: &RetainedCaptureRootV1,
    slot: Slot,
    kind: Kind,
) -> StoreResult<&'a RetainedCaptureLeafV1> {
    let leaf = retained(materials, digest)?;
    if leaf.version != 1 || &leaf.root != root || leaf.slot != slot || leaf.kind != kind {
        return Err(Error::SourceMismatch("documentary body root or slot").into());
    }
    Ok(leaf)
}

fn retained<'a>(
    materials: &'a [Material],
    digest: &AttachmentDigestV1,
) -> StoreResult<&'a RetainedCaptureLeafV1> {
    let material = materials
        .iter()
        .find(|material| &material.reference().digest == digest)
        .ok_or(Error::SourceMismatch("documentary original body absent"))?;
    let Material::Retained { leaf } = material else {
        return Err(Error::SourceMismatch("documentary body is not retained original").into());
    };
    Ok(leaf)
}

fn within_row(
    body: &AttachmentObjectReferenceV1,
    locator: &[u8],
    row: Option<SourceRowLocator>,
) -> StoreResult<bool> {
    let row = row.ok_or(Error::SourceMismatch("grade documentary row absent"))?;
    let (start, end) = locator_range(body, locator)?;
    let row_end = row
        .byte_offset()
        .checked_add(row.byte_length())
        .ok_or(Error::BudgetExceeded)?;
    let row_start = u64::try_from(row.byte_offset()).map_err(|_| Error::BudgetExceeded)?;
    let row_end = u64::try_from(row_end).map_err(|_| Error::BudgetExceeded)?;
    if row.ordinal() == 0 || row.byte_length() == 0 || row_end > body.bytes {
        return Err(Error::SourceMismatch("grade documentary row bounds").into());
    }
    Ok(start >= row_start && end <= row_end)
}
