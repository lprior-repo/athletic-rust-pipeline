use crate::{CrawlError, CrawlResult};
use sha2::{Digest, Sha256};
use std::time::Duration;

mod document;
mod group;
mod locator;
mod packet;
mod recorded;
mod types;

pub use locator::MhsaaCoopTextLocatorV1;
use locator::MhsaaCoopTextLocatorV1 as Locator;
pub use packet::{
    CheckedMhsaaCoopPdfPacketV1, MhsaaCoopPdfCaptureV1, MhsaaCoopPdfPacketRefV1,
    MhsaaCoopPdfPacketV1, MAX_MHSAA_COOP_PDF_PACKET_BYTES, MHSAA_COOP_PDF_CAPTURE_PHASE,
    MHSAA_COOP_PDF_PACKET_INTERPRETATION,
};
pub use recorded::{
    read_recorded_mhsaa_coop_pdf_packet, record_mhsaa_coop_pdf_packet, RecordedMhsaaCoopPdfPacketV1,
};
pub use types::{MhsaaCoopConstituentCandidateV1, MhsaaCoopPdfCandidateV1};
use types::{MhsaaCoopConstituentCandidateV1 as Member, ParsedGroup};

pub const MHSAA_COOPS_SOURCE_URL: &str =
    "https://www.mhsaa.com/sites/default/files/Enrollment%20and%20Classification/hscoop.pdf";
const MAX_PDF_BYTES: usize = 1024 * 1024;
const MAX_TEXT_BYTES: usize = 256 * 1024;
const MAX_PAGES: usize = 64;
const MAX_LINES: usize = 4096;
const MAX_LINE_BYTES: usize = 2048;
const TITLE: &str = "2026-2027 Approved Cooperative Programs - High Schools";

pub fn decode_mhsaa_coop_candidate_v1<'a>(
    source_url: &str,
    original: &'a [u8],
) -> CrawlResult<MhsaaCoopPdfCandidateV1<'a>> {
    validate_original(source_url, original)?;
    let layout = extract_layout(original)?;
    candidate(original, layout)
}

fn validate_original(source_url: &str, original: &[u8]) -> CrawlResult<()> {
    bound(original.len(), MAX_PDF_BYTES, "MHSAA co-op PDF bytes")?;
    if source_url != MHSAA_COOPS_SOURCE_URL || original.is_empty() {
        return Err(invalid("MHSAA co-op source differs from the closed recipe"));
    }
    Ok(())
}

fn extract_layout(original: &[u8]) -> CrawlResult<String> {
    let bytes = crate::convert::converter_stdout_capped(
        "pdftotext",
        ["-layout", "-", "-"],
        original,
        Duration::from_secs(30),
        MAX_TEXT_BYTES,
    )?;
    String::from_utf8(bytes).map_err(|_| invalid("MHSAA co-op layout is not UTF-8"))
}

fn candidate(original: &[u8], layout: String) -> CrawlResult<MhsaaCoopPdfCandidateV1<'_>> {
    let document = document::read(&layout)?;
    Ok(MhsaaCoopPdfCandidateV1 {
        original,
        source_sha256: hash(original),
        layout_sha256: hash(layout.as_bytes()),
        layout,
        school_year: document.school_year,
        heading: document.heading,
        school_year_label: document.school_year_label,
        revision_label: document.revision_label,
        group: document.group,
    })
}

fn invalid(detail: &'static str) -> CrawlError {
    CrawlError::Schema {
        url: MHSAA_COOPS_SOURCE_URL.to_string(),
        detail: detail.to_string(),
    }
}

fn bound(requested: usize, limit: usize, resource: &'static str) -> CrawlResult<()> {
    if requested > limit {
        return Err(CrawlError::Resource {
            resource,
            requested,
            limit,
        });
    }
    Ok(())
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests;
