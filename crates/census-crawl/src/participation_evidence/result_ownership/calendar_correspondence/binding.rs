use super::objects::{invalid, object_reference, Objects};
use crate::participation_evidence::recorded::{
    CheckedDocumentarySourceFactsV1, DiscoveredResultAttachmentV1,
    RecordedIndependentParticipationV1,
};
use crate::participation_evidence::VerifiedIndependentParticipationV1;
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::{
    AttachmentDigestV1, AttachmentRecordReferenceV1, AttachmentReplayGrantV1,
    AttachmentSubjectRoleV1, RetainedDocumentSubjectReferenceV1, ReviewedReferenceRecordV1,
};
use census_store::{
    CurrentEvidenceSubjectAttachmentReferencesV1, DiscoveredAttachmentCaptureV1,
    DiscoveredAttachmentLocatorV1, ReadAttachmentContextObjectV1, RetainedCaptureLeafKindV1,
    RetainedCaptureRootV1, RetainedCaptureSlotV1,
};

pub(super) fn status_binding(
    current: &CurrentEvidenceSubjectAttachmentReferencesV1<'_, '_>,
    target: &census_domain::evidence_subject_attachment::AttachmentTargetV1,
    fact: &CheckedDocumentarySourceFactsV1<'_>,
    recorded: &RecordedIndependentParticipationV1,
    objects: &mut Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<(AttachmentRecordReferenceV1, RetainedCaptureRootV1)> {
    let source = fact.source();
    let discovery =
        current.discover_active_subjects_v1(AttachmentSubjectRoleV1::AthleteStatus, grant)?;
    let item = discovery
        .subjects()
        .iter()
        .try_fold(None, |found, candidate| {
            let matches = candidate.record().record_digest.as_str()
                == fact.attachment_record_digest()
                && candidate.subject().subject.as_str() == source.subject().reference();
            if !matches {
                return Ok(found);
            }
            if found.is_some() {
                return Err(invalid("calendar documentary status binding is ambiguous"));
            }
            Ok(Some(candidate))
        })?
        .ok_or_else(|| invalid("calendar documentary status binding differs"))?;
    fact.require_attached_subject_v1(item, grant)?;
    let bound = current
        .bind_exact_subject(item.subject(), grant)?
        .ok_or_else(|| invalid("calendar documentary status is no longer active"))?;
    bound.require_original_grant(grant)?;
    if bound.record().record_digest.as_str() != fact.attachment_record_digest()
        || bound.store_identity() != current.store_identity()
        || bound.snapshot().sequence() != current.snapshot_sequence()
        || bound.accepted_public_head() != current.accepted_public_head()
        || bound.inventory() != current.inventory()
        || bound.target() != target
    {
        return Err(invalid("calendar documentary status owner moved"));
    }
    let root = retain_status_capture(item, recorded, objects, grant)?;
    retain_locator(item.locator(), &item.subject().locator, objects, grant)?;
    retain_proof_context(source, &item.subject().interpretation, objects, grant)?;
    retain_context(current, item.subject(), bound.record(), objects, grant)?;
    Ok((
        AttachmentRecordReferenceV1 {
            operation: bound.record().command.operation.clone(),
            digest: bound.record().record_digest.clone(),
        },
        root,
    ))
}

fn retain_status_capture(
    item: &census_store::ActiveAttachmentSubjectV1<'_>,
    recorded: &RecordedIndependentParticipationV1,
    objects: &mut Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<RetainedCaptureRootV1> {
    let packet_root = recorded.root();
    let DiscoveredAttachmentCaptureV1::Retained(leaf) = item.capture() else {
        return Err(invalid("calendar documentary capture is not retained"));
    };
    if leaf.object.digest != item.subject().capture {
        return Err(invalid("calendar documentary capture object differs"));
    }
    let context = item.record().command.reviewed_context.objects.as_slice();
    grant
        .charge_steps(context.len())
        .map_err(super::objects::domain)?;
    let body = context
        .iter()
        .find(|reference| reference.digest == item.subject().body)
        .ok_or_else(|| invalid("calendar approved original status body absent"))?;
    if item.subject().namespace.as_str() == "milesplit_result_document_v1" {
        let anchor = context
            .iter()
            .find(|reference| *reference == &packet_root.payload)
            .ok_or_else(|| invalid("calendar approved packet payload anchor absent"))?;
        objects.retain(
            census_store::RetainedCaptureLeafV1 {
                version: 1,
                root: packet_root.clone(),
                slot: RetainedCaptureSlotV1::ParticipationStatus,
                kind: RetainedCaptureLeafKindV1::RootJson,
                object: anchor.clone(),
            },
            grant,
        )?;
    } else if leaf.root != *packet_root
        || leaf.slot != RetainedCaptureSlotV1::ParticipationStatus
        || leaf.kind != RetainedCaptureLeafKindV1::CaptureJson
    {
        return Err(invalid("calendar documentary capture owner differs"));
    }
    objects.retain(leaf.clone(), grant)?;
    objects.retain(
        census_store::RetainedCaptureLeafV1 {
            version: 1,
            root: leaf.root.clone(),
            slot: leaf.slot,
            kind: census_store::RetainedCaptureLeafKindV1::OriginalBody,
            object: body.clone(),
        },
        grant,
    )?;
    Ok(packet_root.clone())
}

pub(super) fn retain_result_context(
    current: &CurrentEvidenceSubjectAttachmentReferencesV1<'_, '_>,
    attached: &DiscoveredResultAttachmentV1<'_, '_>,
    objects: &mut Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<()> {
    let discovery = current.discover_active_result_subjects_v1(grant)?;
    let item = discovery
        .subjects()
        .iter()
        .try_fold(None, |found, candidate| {
            if candidate.subject() != attached.subject() || candidate.record() != attached.record()
            {
                return Ok(found);
            }
            if found.is_some() {
                return Err(invalid("calendar result binding is ambiguous"));
            }
            Ok(Some(candidate))
        })?
        .ok_or_else(|| invalid("calendar result binding differs"))?;
    match item.capture() {
        DiscoveredAttachmentCaptureV1::Retained(leaf)
            if leaf.object.digest == attached.subject().capture =>
        {
            objects.retain(leaf.clone(), grant)?;
        }
        _ => return Err(invalid("calendar result capture is not retained")),
    }
    retain_locator(item.locator(), &item.subject().locator, objects, grant)?;
    retain_context(
        current,
        attached.subject(),
        attached.record(),
        objects,
        grant,
    )
}

fn retain_proof_context(
    source: &VerifiedIndependentParticipationV1,
    expected: &AttachmentDigestV1,
    objects: &mut Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<()> {
    let bytes = super::proof::preimage(source, false)?.into_bytes();
    let reference = object_reference(&bytes, grant)?;
    if reference.digest != *expected
        || reference.digest.as_str() != source.status_proof().interpretation()
    {
        return Err(invalid("status interpretation preimage differs"));
    }
    let held = objects.owned(bytes, grant)?;
    if held != reference {
        return Err(invalid("status interpretation reference differs"));
    }
    Ok(())
}

fn retain_context(
    current: &CurrentEvidenceSubjectAttachmentReferencesV1<'_, '_>,
    subject: &RetainedDocumentSubjectReferenceV1,
    record: &ReviewedReferenceRecordV1,
    objects: &mut Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<()> {
    let context = record.command.reviewed_context.objects.as_slice();
    grant
        .charge_steps(context.len())
        .map_err(super::objects::domain)?;
    let reference = context
        .iter()
        .find(|reference| reference.digest == subject.context)
        .ok_or_else(|| invalid("approved subject context absent"))?;
    if objects.contains(reference) {
        return Ok(());
    }
    match current
        .read_active_context_object_v1(subject, reference, grant)?
        .ok_or_else(|| invalid("approved subject context preimage absent"))?
    {
        ReadAttachmentContextObjectV1::Inline { bytes, .. } => {
            if objects.owned(bytes, grant)? != *reference {
                return Err(invalid("approved subject context reference differs"));
            }
        }
        ReadAttachmentContextObjectV1::Retained(leaf) => {
            objects.retain(leaf.clone(), grant)?;
        }
    }
    Ok(())
}

fn retain_locator(
    locator: DiscoveredAttachmentLocatorV1<'_>,
    expected: &AttachmentDigestV1,
    objects: &mut Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<()> {
    match locator {
        DiscoveredAttachmentLocatorV1::Inline { reference, bytes } => {
            if &reference.digest != expected || object_reference(bytes, grant)? != *reference {
                return Err(invalid("calendar documentary locator differs"));
            }
            let held = objects.bytes(bytes, grant)?;
            if held != *reference {
                return Err(invalid("calendar locator preimage differs"));
            }
        }
        DiscoveredAttachmentLocatorV1::Retained(leaf) => {
            if &leaf.object.digest != expected {
                return Err(invalid("calendar locator differs"));
            }
            objects.retain(leaf.clone(), grant)?;
        }
    }
    Ok(())
}

pub(super) fn retain_calendar_original_body(
    requested: &RecordedIndependentParticipationV1,
    documentary_root: &RetainedCaptureRootV1,
    documentary: &VerifiedIndependentParticipationV1,
    objects: &mut Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<()> {
    let leaf = requested.calendar_original_body();
    if leaf.root != *documentary_root
        || leaf.root != *requested.root()
        || leaf.slot != RetainedCaptureSlotV1::ParticipationSchedule
        || leaf.kind != RetainedCaptureLeafKindV1::OriginalBody
    {
        return Err(invalid(
            "calendar original schedule body descriptor differs",
        ));
    }
    if leaf.object.digest.as_str() != documentary.season_proof().capture() {
        return Err(invalid("calendar original schedule body digest differs"));
    }
    objects.retain(leaf.clone(), grant)
}
