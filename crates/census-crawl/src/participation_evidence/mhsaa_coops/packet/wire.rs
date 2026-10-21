use super::{body, MhsaaCoopPdfCaptureV1, MhsaaCoopPdfPacketV1, MAX_BASE64_BYTES};
use crate::participation_evidence::RetainedHttpProvenanceV1;
use census_store::StoreResult;
use chrono::{DateTime, Utc};
use serde::Deserialize;

mod bounded;
use bounded::Text;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Wire {
    version: u32,
    interpretation: Text<128>,
    capture: Capture,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Capture {
    requested_url: Text<2048>,
    final_url: Text<2048>,
    status: u16,
    acquired_at: DateTime<Utc>,
    body_bytes: usize,
    body_sha256: Text<64>,
    body_base64: Text<MAX_BASE64_BYTES>,
    provenance: Raw,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    request_method: Text<16>,
    #[serde(deserialize_with = "bounded::headers")]
    request_headers: Vec<(String, String)>,
    #[serde(deserialize_with = "bounded::bytes")]
    response_header_bytes: Vec<u8>,
    content_type: Text<128>,
    body_encoding: crate::participation_evidence::RetainedBodyEncodingV1,
    started_at: DateTime<Utc>,
}

impl Wire {
    pub(super) fn into_packet(self) -> StoreResult<MhsaaCoopPdfPacketV1> {
        if self.version != 1 || self.interpretation.0 != super::MHSAA_COOP_PDF_PACKET_INTERPRETATION
        {
            return Err(super::refused(
                "MHSAA co-op PDF packet interpretation differs",
            ));
        }
        let packet = MhsaaCoopPdfPacketV1 {
            version: self.version,
            interpretation: self.interpretation.0,
            capture: self.capture.into_capture()?,
        };
        packet.validate()?;
        Ok(packet)
    }
}

impl Capture {
    fn into_capture(self) -> StoreResult<MhsaaCoopPdfCaptureV1> {
        let provenance = RetainedHttpProvenanceV1 {
            request_method: self.provenance.request_method.0,
            request_headers: self.provenance.request_headers,
            response_header_bytes: self.provenance.response_header_bytes,
            content_type: self.provenance.content_type.0,
            body_encoding: self.provenance.body_encoding,
            started_at: self.provenance.started_at,
        };
        super::provenance::validate(
            &self.requested_url.0,
            &self.final_url.0,
            self.status,
            self.acquired_at,
            self.body_bytes,
            &provenance,
        )?;
        let original = body::decode(&self.body_base64.0, self.body_bytes, &self.body_sha256.0)?;
        Ok(MhsaaCoopPdfCaptureV1 {
            requested_url: self.requested_url.0,
            final_url: self.final_url.0,
            status: self.status,
            acquired_at: self.acquired_at,
            body_bytes: self.body_bytes,
            body_sha256: self.body_sha256.0,
            body_base64: self.body_base64.0,
            provenance,
            original,
        })
    }
}
