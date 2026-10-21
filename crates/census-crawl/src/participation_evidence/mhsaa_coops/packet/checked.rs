use super::{MhsaaCoopPdfPacketRefV1, MhsaaCoopPdfPacketV1};
use crate::participation_evidence::mhsaa_coops::MhsaaCoopPdfCandidateV1;

pub struct CheckedMhsaaCoopPdfPacketV1<'a> {
    packet: &'a MhsaaCoopPdfPacketV1,
    reference: MhsaaCoopPdfPacketRefV1,
    candidate: MhsaaCoopPdfCandidateV1<'a>,
}

impl<'a> CheckedMhsaaCoopPdfPacketV1<'a> {
    pub(super) fn new(
        packet: &'a MhsaaCoopPdfPacketV1,
        reference: MhsaaCoopPdfPacketRefV1,
        candidate: MhsaaCoopPdfCandidateV1<'a>,
    ) -> Self {
        Self {
            packet,
            reference,
            candidate,
        }
    }

    pub fn packet(&self) -> &'a MhsaaCoopPdfPacketV1 {
        self.packet
    }
    pub fn reference(&self) -> &MhsaaCoopPdfPacketRefV1 {
        &self.reference
    }
    pub fn layout_recipe(&self) -> &'static str {
        "pdftotext -layout - -"
    }
    pub fn candidate(&self) -> &MhsaaCoopPdfCandidateV1<'a> {
        &self.candidate
    }
}
