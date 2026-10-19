use super::*;
use crate::evidence_subject_attachment::external::CheckedRoot;
use crate::{
    RetainedCaptureLeafKindV1 as Kind, RetainedCaptureRootV1, RetainedCaptureSlotV1 as Slot,
};
use census_domain::evidence_subject_attachment::{
    AttachmentDigestV1, RetainedDocumentSubjectReferenceV1,
};
use census_domain::model::{Gender, Sport};

pub(super) fn require(
    view: &impl RetainedJournalReadSource,
    source: &ResultCalendarSourceClaimV1,
    materials: &[Material],
    record: &ReviewedReferenceRecordV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    if source.sport != Sport::OutdoorTrack || source.category != Gender::Boys {
        return Err(Error::SourceMismatch("unsupported documentary parser scope").into());
    }
    let root = super::root(
        materials,
        &source.calendar.documentary_root,
        crate::RetainedCaptureRootSchemaV1::ParticipationPacketV1,
        grant,
    )?;
    let subject = record
        .command
        .subjects
        .as_slice()
        .iter()
        .find(|subject| {
            subject.role == AttachmentSubjectRoleV1::AthleteStatus
                && subject.context == source.calendar.documentary_root.digest
        })
        .ok_or(Error::SourceMismatch("original documentary subject"))?;
    original_leaves(source, materials, subject, &root)?;
    crate::evidence_subject_attachment::external::with_checked(
        view,
        &root,
        grant,
        |_, checked, grant| {
            let CheckedRoot::Participation { status, schedule } = checked else {
                return Err(Error::SourceMismatch("documentary packet schema").into());
            };
            if status.body_sha256 != subject.body.as_str()
                || schedule.body_sha256 != source.calendar.capture.as_str()
            {
                return Err(Error::SourceMismatch("documentary original capture binding").into());
            }
            interpretation(source, materials, grant)
        },
    )
}

fn original_leaves(
    source: &ResultCalendarSourceClaimV1,
    materials: &[Material],
    subject: &RetainedDocumentSubjectReferenceV1,
    root: &RetainedCaptureRootV1,
) -> StoreResult<()> {
    leaf(
        materials,
        &subject.capture,
        root,
        Slot::ParticipationStatus,
        Kind::CaptureJson,
    )?;
    leaf(
        materials,
        &subject.body,
        root,
        Slot::ParticipationStatus,
        Kind::OriginalBody,
    )?;
    leaf(
        materials,
        &source.calendar.capture,
        root,
        Slot::ParticipationSchedule,
        Kind::OriginalBody,
    )?;
    Ok(())
}

fn leaf<'a>(
    materials: &'a [Material],
    digest: &AttachmentDigestV1,
    root: &RetainedCaptureRootV1,
    slot: Slot,
    kind: Kind,
) -> StoreResult<&'a AttachmentObjectReferenceV1> {
    let material = materials
        .iter()
        .find(|material| &material.reference().digest == digest)
        .ok_or(Error::SourceMismatch("documentary original body absent"))?;
    let Material::Retained { leaf } = material else {
        return Err(Error::SourceMismatch("documentary body is not retained original").into());
    };
    if leaf.version != 1 || &leaf.root != root || leaf.slot != slot || leaf.kind != kind {
        return Err(Error::SourceMismatch("documentary body root or slot").into());
    }
    Ok(&leaf.object)
}

fn interpretation(
    source: &ResultCalendarSourceClaimV1,
    materials: &[Material],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let locator = inline(materials, &source.calendar.locator, 96, grant)?;
    let text = std::str::from_utf8(&locator)
        .map_err(|_| Error::SourceMismatch("calendar locator UTF-8"))?;
    let (start, end) = text
        .strip_prefix("html_bytes:")
        .and_then(|text| text.split_once(':'))
        .ok_or(Error::SourceMismatch("calendar locator shape"))?;
    let start: u64 = start
        .parse()
        .map_err(|_| Error::SourceMismatch("calendar locator start"))?;
    let end: u64 = end
        .parse()
        .map_err(|_| Error::SourceMismatch("calendar locator end"))?;
    let body = materials
        .iter()
        .find(|material| material.reference().digest == source.calendar.capture)
        .ok_or(Error::SourceMismatch("calendar original body reference"))?;
    if start >= end || end > body.reference().bytes {
        return Err(Error::SourceMismatch("calendar locator byte range").into());
    }
    let interpretation = inline(materials, &source.calendar.interpretation, 256, grant)?;
    let expected = [
        b"mhsaa_subject_dcc_event_day_v1".as_slice(),
        b"\n",
        source.calendar.capture.as_str().as_bytes(),
        b"\n",
        locator.as_slice(),
    ];
    if !expected.into_iter().flatten().eq(interpretation.iter()) {
        return Err(Error::SourceMismatch("calendar interpretation preimage").into());
    }
    Ok(())
}

fn inline(
    materials: &[Material],
    digest: &AttachmentDigestV1,
    maximum: u64,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    let material = materials
        .iter()
        .find(|material| &material.reference().digest == digest)
        .ok_or(Error::SourceMismatch("calendar required preimage absent"))?;
    if material.reference().bytes > maximum {
        return Err(Error::SourceMismatch("calendar preimage bound").into());
    }
    material.inline_bytes(grant)
}
