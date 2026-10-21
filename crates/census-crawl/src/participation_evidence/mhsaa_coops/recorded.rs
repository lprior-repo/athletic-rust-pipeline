use super::packet::{
    MhsaaCoopPdfPacketRefV1, MhsaaCoopPdfPacketV1, MAX_MHSAA_COOP_PDF_PACKET_BYTES,
};
use super::CheckedMhsaaCoopPdfPacketV1;
use crate::participation_evidence::recorded::with_grant;
use crate::recording::RowSink;
use crate::{CrawlResult, Recording};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_store::{
    EvidenceSubjectAttachmentStoreError, RetainedCaptureRootSchemaV1, RetainedCaptureRootV1, Store,
    StoreError, StoreSnapshot,
};

pub struct RecordedMhsaaCoopPdfPacketV1 {
    packet: MhsaaCoopPdfPacketV1,
    root: RetainedCaptureRootV1,
}

impl RecordedMhsaaCoopPdfPacketV1 {
    pub fn packet(&self) -> &MhsaaCoopPdfPacketV1 {
        &self.packet
    }
    pub fn root(&self) -> &RetainedCaptureRootV1 {
        &self.root
    }

    pub fn read(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> CrawlResult<CheckedMhsaaCoopPdfPacketV1<'_>> {
        with_grant(grant, |grant| {
            grant
                .charge_bytes(super::MAX_TEXT_BYTES)
                .map_err(grant_error)?;
            grant.charge_steps(1).map_err(grant_error)?;
            self.packet.read()
        })
    }
}

pub fn record_mhsaa_coop_pdf_packet(
    store: &Store,
    recording: &Recording,
    packet: &MhsaaCoopPdfPacketV1,
) -> CrawlResult<MhsaaCoopPdfPacketRefV1> {
    let checked = packet.read()?;
    let reference = checked.reference().clone();
    let payload = serde_json::to_value(packet).map_err(super::super::json)?;
    let schema = RetainedCaptureRootSchemaV1::MhsaaCoopPdfPacketV1;
    Store::check_retained_capture_entry_v1(schema, reference.as_str(), &payload)?;
    let mut batch = RowSink::Record { store, recording }.write_batch();
    batch.journal_done(schema.phase(), reference.as_str(), &payload)?;
    batch.commit()?;
    Ok(reference)
}

pub fn read_recorded_mhsaa_coop_pdf_packet(
    snapshot: &StoreSnapshot<'_>,
    reference: &MhsaaCoopPdfPacketRefV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<Option<RecordedMhsaaCoopPdfPacketV1>> {
    with_grant(grant, |grant| {
        let Some(held) = snapshot.read_retained_capture_root_v1(
            RetainedCaptureRootSchemaV1::MhsaaCoopPdfPacketV1,
            reference.as_str(),
            grant,
        )?
        else {
            return Ok(None);
        };
        let packet = decode_payload(held.payload().get().as_bytes(), reference, grant)?;
        Ok(Some(RecordedMhsaaCoopPdfPacketV1 {
            packet,
            root: held.root().clone(),
        }))
    })
}

fn decode_payload(
    bytes: &[u8],
    reference: &MhsaaCoopPdfPacketRefV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<MhsaaCoopPdfPacketV1> {
    with_grant(grant, |grant| {
        super::super::bound(
            bytes.len(),
            MAX_MHSAA_COOP_PDF_PACKET_BYTES,
            "recorded MHSAA PDF packet bytes",
        )?;
        let charge = bytes
            .len()
            .checked_mul(4)
            .ok_or(StoreError::CounterOverflow)?;
        grant.charge_bytes(charge).map_err(grant_error)?;
        grant.charge_steps(1).map_err(grant_error)?;
        let packet = MhsaaCoopPdfPacketV1::decode(bytes)?;
        if packet.reference()? != *reference {
            return Err(super::super::invalid(
                "recorded MHSAA PDF packet differs from its typed key",
            )
            .into());
        }
        Ok(packet)
    })
}

fn grant_error(
    error: census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError,
) -> StoreError {
    EvidenceSubjectAttachmentStoreError::Domain(error).into()
}

#[cfg(test)]
mod tests;
