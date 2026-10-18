use super::super::super::attached_result::view::View;
use super::super::super::window::read_window_from;
use super::super::super::RecordedIndependentParticipationWindowV1;
use super::super::bytes;
use crate::participation_evidence::{invalid, ParticipationEvidenceRefV1};
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1 as Grant;
use census_store::{
    ActiveAttachmentSubjectV1, DiscoveredAttachmentCaptureV1, RetainedCaptureLeafKindV1,
    RetainedCaptureRootSchemaV1, RetainedCaptureSlotV1,
};

pub(super) fn supported(item: &ActiveAttachmentSubjectV1<'_>) -> bool {
    matches!(item.capture(), DiscoveredAttachmentCaptureV1::Retained(leaf)
        if leaf.root.schema == RetainedCaptureRootSchemaV1::ParticipationWindowPacketV1
        && leaf.slot == RetainedCaptureSlotV1::ParticipationWindow
        && leaf.kind == RetainedCaptureLeafKindV1::RootJson)
        && item.subject().namespace.as_str() == "mhsaa_article_v1"
}

pub(super) fn packet(
    view: View<'_, '_, '_>,
    item: &ActiveAttachmentSubjectV1<'_>,
    grant: &mut Grant,
) -> CrawlResult<RecordedIndependentParticipationWindowV1> {
    let DiscoveredAttachmentCaptureV1::Retained(leaf) = item.capture() else {
        return Err(
            invalid("attached window requires retained participation window capture").into(),
        );
    };
    if leaf.slot != RetainedCaptureSlotV1::ParticipationWindow
        || leaf.kind != RetainedCaptureLeafKindV1::RootJson
        || leaf.root.schema != RetainedCaptureRootSchemaV1::ParticipationWindowPacketV1
        || leaf.object.digest != item.subject().capture
    {
        return Err(invalid("attached window retained source owner differs").into());
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
    let reference = ParticipationEvidenceRefV1::parse(&leaf.root.key)?;
    let recorded = read_window_from(
        view.source(),
        &reference,
        || Ok(store.clone()),
        sequence,
        grant,
    )?
    .ok_or_else(|| invalid("attached window retained packet is missing"))?;
    if recorded.root() != &leaf.root {
        return Err(invalid("attached window packet root differs").into());
    }
    Ok(recorded)
}
