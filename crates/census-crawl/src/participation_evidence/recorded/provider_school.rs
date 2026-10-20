use super::super::provider_school::{
    CheckedProviderSchoolPacketV1, ProviderSchoolPacketRefV1, ProviderSchoolPacketV1,
};
use super::super::{bound, codec, invalid, json, MAX_PACKET};
use super::read::{retained, with_grant};
use crate::recording::RowSink;
use crate::{CrawlResult, Recording};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{SchoolId, SourceObservation};
use census_store::{
    EvidenceSubjectAttachmentStoreError, RetainedCaptureRootSchemaV1, RetainedCaptureRootV1,
    RosterSourceEvidenceV1, Store, StoreError, StoreSnapshot, Table,
};

pub struct RecordedProviderSchoolV1 {
    checked: CheckedProviderSchoolPacketV1,
    root: RetainedCaptureRootV1,
}

impl RecordedProviderSchoolV1 {
    pub fn checked(&self) -> &CheckedProviderSchoolPacketV1 {
        &self.checked
    }
    pub fn root(&self) -> &RetainedCaptureRootV1 {
        &self.root
    }
}

pub fn record_provider_school_intake(
    store: &Store,
    recording: &Recording,
    packet: &ProviderSchoolPacketV1,
) -> CrawlResult<SchoolId> {
    let checked = packet.read()?;
    packet.encode()?;
    let payload = serde_json::to_value(packet).map_err(json)?;
    let schema = RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1;
    Store::check_retained_capture_entry_v1(schema, checked.reference().as_str(), &payload)?;
    let observation = SourceObservation::School(checked.observation().clone());
    codec::encode(checked.school(), MAX_PACKET)?;
    codec::encode(&observation, MAX_PACKET)?;
    let mut batch = RowSink::Record { store, recording }.write_batch();
    batch.journal_done(schema.phase(), checked.reference().as_str(), &payload)?;
    batch.append_many(Table::Schools, std::slice::from_ref(checked.school()))?;
    batch.append_many(
        Table::SourceObservations,
        std::slice::from_ref(&observation),
    )?;
    batch.commit()?;
    Ok(checked.school().id.clone())
}

pub fn read_recorded_provider_school(
    snapshot: &StoreSnapshot<'_>,
    reference: &ProviderSchoolPacketRefV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<Option<RecordedProviderSchoolV1>> {
    with_grant(grant, |grant| {
        let Some((packet, held)) = retained::<ProviderSchoolPacketV1>(
            snapshot,
            RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1,
            reference.as_str(),
            grant,
        )?
        else {
            return Ok(None);
        };
        let checked = packet.read()?;
        if checked.reference() != reference {
            return Err(
                invalid("recorded provider school packet differs from its typed key").into(),
            );
        }
        let root = held.root().clone();
        Ok(Some(RecordedProviderSchoolV1 { checked, root }))
    })
}

pub fn read_retained_provider_roster_body_v1(
    evidence: &RosterSourceEvidenceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<Vec<u8>> {
    with_grant(grant, |grant| {
        let held = evidence.retained_root(grant)?;
        if held.root().schema != RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1 {
            return Err(invalid("roster body requires the retained provider-school root").into());
        }
        let raw = held.payload().get().as_bytes();
        bound(
            raw.len(),
            MAX_PACKET,
            "retained provider-school body packet",
        )?;
        let decode_charge = raw
            .len()
            .checked_mul(2)
            .ok_or(StoreError::CounterOverflow)?;
        grant
            .charge_bytes(decode_charge)
            .map_err(EvidenceSubjectAttachmentStoreError::Domain)
            .map_err(StoreError::from)?;
        grant
            .charge_steps(1)
            .map_err(EvidenceSubjectAttachmentStoreError::Domain)
            .map_err(StoreError::from)?;
        let packet: ProviderSchoolPacketV1 = serde_json::from_slice(raw).map_err(json)?;
        if packet.reference()?.as_str() != held.root().key {
            return Err(invalid("roster body packet differs from its retained key").into());
        }
        Ok(packet.roster().body()?.into_bytes())
    })
}
