use super::{decode_mhsaa_coop_candidate_v1, MHSAA_COOPS_SOURCE_URL};
use crate::CrawlResult;
use census_store::StoreResult;
use serde::Serialize;

mod body;
mod capture;
mod checked;
mod provenance;
mod reference;
mod wire;

pub use capture::MhsaaCoopPdfCaptureV1;
pub use checked::CheckedMhsaaCoopPdfPacketV1;
pub use reference::MhsaaCoopPdfPacketRefV1;

pub const MHSAA_COOP_PDF_CAPTURE_PHASE: &str = "mhsaa_coop_pdf_capture_v1";
pub const MHSAA_COOP_PDF_PACKET_INTERPRETATION: &str = "mhsaa_approved_coop_pdf_v1";
pub const MAX_MHSAA_COOP_PDF_PACKET_BYTES: usize = 2 * 1024 * 1024;
const MAX_BASE64_BYTES: usize = 1_398_104;

#[derive(Debug, Serialize)]
pub struct MhsaaCoopPdfPacketV1 {
    version: u32,
    interpretation: String,
    capture: MhsaaCoopPdfCaptureV1,
}

impl MhsaaCoopPdfPacketV1 {
    pub fn new(capture: MhsaaCoopPdfCaptureV1) -> StoreResult<Self> {
        let packet = Self {
            version: 1,
            interpretation: MHSAA_COOP_PDF_PACKET_INTERPRETATION.into(),
            capture,
        };
        packet.encode()?;
        Ok(packet)
    }

    pub fn decode(bytes: &[u8]) -> StoreResult<Self> {
        limit(
            bytes.len(),
            MAX_MHSAA_COOP_PDF_PACKET_BYTES,
            "MHSAA co-op PDF packet bytes",
        )?;
        let wire: wire::Wire = serde_json::from_slice(bytes).map_err(super::super::json)?;
        let packet = wire.into_packet()?;
        if packet.encode()? != bytes {
            return Err(refused("MHSAA co-op PDF packet is not canonical"));
        }
        Ok(packet)
    }

    pub fn encode(&self) -> StoreResult<Vec<u8>> {
        self.validate()?;
        let payload = serde_json::to_value(self).map_err(super::super::json)?;
        super::super::codec::encode(&payload, MAX_MHSAA_COOP_PDF_PACKET_BYTES)
    }

    pub fn reference(&self) -> StoreResult<MhsaaCoopPdfPacketRefV1> {
        MhsaaCoopPdfPacketRefV1::parse(&super::hash(&self.encode()?))
    }

    pub fn capture(&self) -> &MhsaaCoopPdfCaptureV1 {
        &self.capture
    }

    pub fn original_body_reference(
        &self,
    ) -> StoreResult<census_domain::evidence_subject_attachment::AttachmentObjectReferenceV1> {
        self.capture.original_body_reference()
    }

    pub fn read(&self) -> CrawlResult<CheckedMhsaaCoopPdfPacketV1<'_>> {
        self.validate()?;
        Ok(CheckedMhsaaCoopPdfPacketV1::new(
            self,
            self.reference()?,
            decode_mhsaa_coop_candidate_v1(
                self.capture.requested_url(),
                self.capture.original_bytes(),
            )?,
        ))
    }

    fn validate(&self) -> StoreResult<()> {
        if self.version != 1 || self.interpretation != MHSAA_COOP_PDF_PACKET_INTERPRETATION {
            return Err(refused("MHSAA co-op PDF packet interpretation differs"));
        }
        self.capture.validate()
    }
}

fn refused(detail: &'static str) -> census_store::StoreError {
    super::super::invalid(detail)
}

fn limit(requested: usize, maximum: usize, what: &'static str) -> StoreResult<()> {
    super::super::bound(requested, maximum, what)
}

#[cfg(test)]
pub(super) mod tests;
