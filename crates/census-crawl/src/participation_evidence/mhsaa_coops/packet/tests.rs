use super::*;
use crate::participation_evidence::{RetainedBodyEncodingV1, RetainedHttpProvenanceV1};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const PDF: &[u8] = include_bytes!("../tests/fixtures/official-approved-coops-2026-27.pdf");
const HEADERS: &[u8] = include_bytes!("tests/official-approved-coops.headers");
const PDF_SHA: &str = "80fe9a2f50949b90143212dbc505452f1a4d15f1ccf9ddd52215fbb73008c7d4";

mod refusals;

#[derive(Deserialize)]
struct Research {
    requested_url: String,
    method: String,
    started_at: DateTime<Utc>,
    completed_at: DateTime<Utc>,
    request_headers: Vec<(String, String)>,
    body_sha256: String,
}

fn research() -> TestResult<Research> {
    Ok(serde_json::from_slice(include_bytes!(
        "tests/official-approved-coops.research.json"
    ))?)
}

fn provenance() -> TestResult<RetainedHttpProvenanceV1> {
    let raw = research()?;
    Ok(RetainedHttpProvenanceV1 {
        request_method: raw.method,
        request_headers: raw.request_headers,
        response_header_bytes: HEADERS.to_vec(),
        content_type: "application/pdf".into(),
        body_encoding: RetainedBodyEncodingV1::Identity,
        started_at: raw.started_at,
    })
}

pub(in crate::participation_evidence::mhsaa_coops) fn packet() -> TestResult<MhsaaCoopPdfPacketV1> {
    let raw = research()?;
    check!(eq; raw.body_sha256, PDF_SHA);
    let capture = MhsaaCoopPdfCaptureV1::new(
        &raw.requested_url,
        &raw.requested_url,
        200,
        raw.completed_at,
        PDF,
        provenance()?,
    )?;
    Ok(MhsaaCoopPdfPacketV1::new(capture)?)
}

#[test]
fn genuine_pdf_packet_preserves_binary_raw_provenance_and_candidate_locators() -> TestResult {
    let packet = packet()?;
    let encoded = packet.encode()?;
    let reopened = MhsaaCoopPdfPacketV1::decode(&encoded)?;
    check!(eq; reopened.encode()?, encoded);
    check!(eq; reopened.capture().original_bytes(), PDF);
    check!(eq; reopened.capture().body_sha256(), PDF_SHA);
    check!(eq; reopened.capture().provenance().response_header_bytes.as_slice(), HEADERS);
    check!(eq; serde_json::to_value(reopened.capture().provenance())?, serde_json::to_value(provenance()?)?);
    let reference = reopened.reference()?;
    check!(eq; reference.as_str(), super::super::hash(&encoded));
    let body = reopened.original_body_reference()?;
    check!(eq; body.digest.as_str(), PDF_SHA);
    check!(eq; body.bytes, 123213);
    let checked = reopened.read()?;
    check!(eq; checked.reference(), &reference);
    check!(eq; checked.layout_recipe(), "pdftotext -layout - -");
    check!(eq; checked.candidate().primary().name(), "Pickford");
    check!(eq; checked.candidate().partner().name(), "Kinross Maplewood Baptist");
    check!(eq; checked.candidate().program_row().page(), 24);
    check!(eq; checked.candidate().source_sha256(), PDF_SHA);
    check!(ne; checked.candidate().layout_sha256(), PDF_SHA);
    Ok(())
}

#[test]
fn literal_packet_shape_uses_pdf_capture_and_unchanged_raw_provenance() -> TestResult {
    let packet = packet()?;
    let value: Value = serde_json::from_slice(&packet.encode()?)?;
    let root = value.as_object().ok_or("packet object")?;
    check!(eq; root.keys().map(String::as_str).collect::<Vec<_>>(), vec!["capture", "interpretation", "version"]);
    check!(eq; root.get("version"), Some(&Value::from(1)));
    check!(eq; root.get("interpretation"), Some(&Value::from("mhsaa_approved_coop_pdf_v1")));
    let capture = value
        .get("capture")
        .and_then(Value::as_object)
        .ok_or("capture object")?;
    check!(eq; capture.keys().map(String::as_str).collect::<Vec<_>>(), vec![
        "acquired_at", "body_base64", "body_bytes", "body_sha256", "final_url", "provenance", "requested_url", "status"]);
    check!(eq; capture.get("provenance"), Some(&serde_json::to_value(provenance()?)?));
    check!(crate::participation_evidence::RetainedHtmlCaptureV1::new(
        MHSAA_COOPS_SOURCE_URL,
        MHSAA_COOPS_SOURCE_URL,
        200,
        packet.capture().acquired_at(),
        PDF,
        provenance()?
    )
    .is_err());
    Ok(())
}

fn altered(change: impl FnOnce(&mut Value)) -> TestResult<Vec<u8>> {
    let mut value = serde_json::to_value(packet()?)?;
    change(&mut value);
    Ok(serde_json::to_vec(&value)?)
}
