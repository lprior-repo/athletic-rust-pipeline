use super::*;
use census_domain::evidence_subject_attachment::AttachmentSubjectRoleV1;
use census_store::{
    ActiveAttachmentSubjectV1, DiscoveredAttachmentCaptureV1, RetainedCaptureLeafKindV1,
    RetainedCaptureRootSchemaV1, RetainedCaptureSlotV1,
};
mod window;

pub(super) fn sources<'a, 'v, 's>(
    view: View<'a, 'v, 's>,
    grant: &mut Grant,
) -> CrawlResult<AttachedParticipationV1<'a, 's>> {
    let metadata = view.discover_role(AttachmentSubjectRoleV1::AthleteStatus, grant)?;
    let mut sources = Vec::new();
    let mut unsupported = Vec::new();
    let mut windows = Vec::new();
    for item in metadata.subjects() {
        steps(grant, 1)?;
        if window::supported(item) {
            let recorded = window::packet(view, item, grant)?;
            subject::require_window(item, recorded.checked(), grant)?;
            let binding = bind(view, item, grant)?;
            reserve(&mut windows, grant)?;
            windows.push(CheckedAttachedWindowV1 { recorded, binding });
            continue;
        }
        if !supported(item) {
            let binding = bind(view, item, grant)?;
            reserve(&mut unsupported, grant)?;
            unsupported.push(binding);
            continue;
        }
        let recorded = packet(view, item, grant)?;
        subject::require(item, recorded.checked(), grant)?;
        let binding = bind(view, item, grant)?;
        reserve(&mut sources, grant)?;
        sources.push(CheckedAttachedStatusV1 { recorded, binding });
    }
    Ok(AttachedParticipationV1 {
        sources,
        windows,
        unsupported,
    })
}

fn packet(
    view: View<'_, '_, '_>,
    item: &ActiveAttachmentSubjectV1<'_>,
    grant: &mut Grant,
) -> CrawlResult<super::super::RecordedIndependentParticipationV1> {
    if item.subject().namespace.as_str() == "milesplit_result_document_v1" {
        return packet_from_approved_anchors(view, item, grant);
    }
    let DiscoveredAttachmentCaptureV1::Retained(leaf) = item.capture() else {
        return Err(invalid("attached status requires retained participation capture").into());
    };
    if leaf.slot != RetainedCaptureSlotV1::ParticipationStatus
        || leaf.kind != RetainedCaptureLeafKindV1::CaptureJson
        || leaf.root.schema != RetainedCaptureRootSchemaV1::ParticipationPacketV1
        || leaf.object.digest != item.subject().capture
    {
        return Err(invalid("attached status retained source owner differs").into());
    }
    match view {
        View::Current(owner) => owner
            .snapshot()
            .verify_retained_capture_leaf_v1(leaf, grant)?,
        View::Frozen(owner) => owner
            .source()
            .verify_retained_capture_leaf_v1(leaf, grant)?,
    }
    let (store, sequence) = match view {
        View::Current(owner) => (owner.store_identity(), owner.snapshot_sequence()),
        View::Frozen(owner) => (owner.store_identity(), owner.snapshot_sequence()),
    };
    bytes(
        grant,
        store
            .as_str()
            .len()
            .checked_add(leaf.root.key.len())
            .ok_or(census_store::StoreError::CounterOverflow)?,
    )?;
    let reference =
        crate::participation_evidence::ParticipationEvidenceRefV1::parse(&leaf.root.key)?;
    let recorded = super::super::read::read_independent_from(
        view.source(),
        &reference,
        || Ok(store.clone()),
        sequence,
        grant,
    )?
    .ok_or_else(|| invalid("attached status retained packet is missing"))?;
    if recorded.root() != &leaf.root {
        return Err(invalid("attached status packet root differs").into());
    }
    Ok(recorded)
}

fn packet_from_approved_anchors(
    view: View<'_, '_, '_>,
    item: &ActiveAttachmentSubjectV1<'_>,
    grant: &mut Grant,
) -> CrawlResult<super::super::RecordedIndependentParticipationV1> {
    let objects = item.record().command.reviewed_context.objects.as_slice();
    steps(grant, objects.len())?;
    let capture = objects
        .iter()
        .find(|reference| reference.digest == item.subject().capture)
        .ok_or_else(|| invalid("approved status capture reference is absent"))?;
    let mut matching = None;
    for candidate in item.retained_root_anchors() {
        steps(grant, 1)?;
        if candidate.root.schema != RetainedCaptureRootSchemaV1::ParticipationPacketV1
            || candidate.slot != RetainedCaptureSlotV1::ParticipationStatus
            || candidate.kind != RetainedCaptureLeafKindV1::RootJson
        {
            continue;
        }
        match view {
            View::Current(owner) => owner
                .snapshot()
                .verify_retained_capture_leaf_v1(candidate, grant)?,
            View::Frozen(owner) => owner
                .source()
                .verify_retained_capture_leaf_v1(candidate, grant)?,
        }
        let (store, sequence) = match view {
            View::Current(owner) => (owner.store_identity(), owner.snapshot_sequence()),
            View::Frozen(owner) => (owner.store_identity(), owner.snapshot_sequence()),
        };
        bytes(
            grant,
            store
                .as_str()
                .len()
                .checked_add(candidate.root.key.len())
                .ok_or(census_store::StoreError::CounterOverflow)?,
        )?;
        let reference =
            crate::participation_evidence::ParticipationEvidenceRefV1::parse(&candidate.root.key)?;
        let recorded = super::super::read::read_independent_from(
            view.source(),
            &reference,
            || Ok(store.clone()),
            sequence,
            grant,
        )?
        .ok_or_else(|| invalid("participation packet is missing"))?;
        if recorded.root() != &candidate.root {
            return Err(invalid("participation packet root differs").into());
        }
        if recorded.checked().status_proof().capture() != item.subject().body.as_str()
            || recorded.checked().subject().reference() != item.subject().subject.as_str()
        {
            continue;
        }
        let capture_leaf = census_store::RetainedCaptureLeafV1 {
            version: 1,
            root: candidate.root.clone(),
            slot: RetainedCaptureSlotV1::ParticipationStatus,
            kind: RetainedCaptureLeafKindV1::CaptureJson,
            object: capture.clone(),
        };
        match view {
            View::Current(owner) => owner
                .snapshot()
                .verify_retained_capture_leaf_v1(&capture_leaf, grant)?,
            View::Frozen(owner) => owner
                .source()
                .verify_retained_capture_leaf_v1(&capture_leaf, grant)?,
        }
        subject::require(item, recorded.checked(), grant)?;
        if matching.replace(recorded).is_some() {
            return Err(invalid("participation packet selection is ambiguous").into());
        }
    }
    matching.ok_or_else(|| invalid("approved participation packet subject is absent").into())
}

fn bind<'a, 'v, 's>(
    view: View<'a, 'v, 's>,
    item: &ActiveAttachmentSubjectV1<'_>,
    grant: &mut Grant,
) -> CrawlResult<StatusBinding<'a, 's>> {
    let bound = match view {
        View::Current(owner) => StatusBinding::Current(
            owner
                .bind_exact_subject(item.subject(), grant)?
                .ok_or_else(|| invalid("attached status is no longer active"))?,
        ),
        View::Frozen(owner) => StatusBinding::Frozen(
            owner
                .bind_exact_subject(item.subject(), grant)?
                .ok_or_else(|| invalid("frozen attached status is no longer active"))?,
        ),
    };
    bound.require_original_grant(grant)?;
    Ok(bound)
}

fn supported(item: &ActiveAttachmentSubjectV1<'_>) -> bool {
    if item.subject().namespace.as_str() == "milesplit_result_document_v1" {
        for anchor in item.retained_root_anchors() {
            if anchor.root.schema == RetainedCaptureRootSchemaV1::ParticipationPacketV1
                && anchor.slot == RetainedCaptureSlotV1::ParticipationStatus
                && anchor.kind == RetainedCaptureLeafKindV1::RootJson
            {
                return true;
            }
        }
        return false;
    }
    matches!(item.capture(), DiscoveredAttachmentCaptureV1::Retained(leaf)
        if leaf.root.schema == RetainedCaptureRootSchemaV1::ParticipationPacketV1
        && leaf.slot == RetainedCaptureSlotV1::ParticipationStatus
        && leaf.kind == RetainedCaptureLeafKindV1::CaptureJson)
        && (item.subject().namespace.as_str() == "mhsaa_article_v1"
            || item.subject().namespace.as_str() == "milesplit_result_document_v1")
}
