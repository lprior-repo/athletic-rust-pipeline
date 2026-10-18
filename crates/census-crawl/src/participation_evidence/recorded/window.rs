use super::super::{
    invalid, IndependentParticipationWindowPacketV1, ParticipationEvidenceRefV1,
    VerifiedIndependentParticipationWindowV1,
};
use super::attached_result::view::Source;
use crate::recording::RowSink;
use crate::{CrawlResult, Recording};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::StoreIdentity;
use census_store::{
    RetainedCaptureRootSchemaV1, RetainedCaptureRootV1, Store, StoreResult, StoreSnapshot,
};

pub struct RecordedIndependentParticipationWindowV1 {
    checked: VerifiedIndependentParticipationWindowV1,
    root: RetainedCaptureRootV1,
}

impl RecordedIndependentParticipationWindowV1 {
    pub fn checked(&self) -> &VerifiedIndependentParticipationWindowV1 {
        &self.checked
    }

    pub fn root(&self) -> &RetainedCaptureRootV1 {
        &self.root
    }
}

pub fn record_independent_participation_window_packet(
    store: &Store,
    recording: &Recording,
    packet: &IndependentParticipationWindowPacketV1,
) -> CrawlResult<()> {
    let snapshot = store.snapshot();
    let checked = packet.read(snapshot.store_identity()?, snapshot.sequence())?;
    let payload = packet.payload()?;
    let schema = RetainedCaptureRootSchemaV1::ParticipationWindowPacketV1;
    Store::check_retained_capture_entry_v1(schema, checked.reference().as_str(), &payload)?;
    let mut batch = RowSink::Record { store, recording }.write_batch();
    batch.journal_done(schema.phase(), checked.reference().as_str(), &payload)?;
    batch.commit()
}

pub fn read_recorded_independent_participation_window(
    snapshot: &StoreSnapshot<'_>,
    reference: &ParticipationEvidenceRefV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<Option<RecordedIndependentParticipationWindowV1>> {
    read_window_from(
        Source::Current(snapshot),
        reference,
        || snapshot.store_identity(),
        snapshot.sequence(),
        grant,
    )
}

pub(super) fn read_window_from(
    source: Source<'_, '_>,
    reference: &ParticipationEvidenceRefV1,
    store_identity: impl FnOnce() -> StoreResult<StoreIdentity>,
    snapshot_sequence: u64,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<Option<RecordedIndependentParticipationWindowV1>> {
    super::read::with_grant(grant, |grant| {
        let Some((packet, held)) =
            super::read::retained_from::<IndependentParticipationWindowPacketV1>(
                source,
                RetainedCaptureRootSchemaV1::ParticipationWindowPacketV1,
                reference.as_str(),
                grant,
            )?
        else {
            return Ok(None);
        };
        let checked = packet.read(store_identity()?, snapshot_sequence)?;
        if checked.reference() != reference {
            return Err(invalid("recorded participation window differs from its typed key").into());
        }
        Ok(Some(RecordedIndependentParticipationWindowV1 {
            checked,
            root: held.root().clone(),
        }))
    })
}
