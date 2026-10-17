use super::*;
use census_store::{JournalReadBudget, RetainedCaptureLeafV1, RetainedCaptureLeafKindV1, RetainedCaptureSlotV1};
use crate::participation_evidence::recorded::{read_recorded_independent_participation, RecordedIndependentParticipationV1};
use serde_json::value::RawValue;
use std::collections::BTreeMap;

pub(super) fn packet(index: usize, schedule: &RetainedHtmlCaptureV1) -> TestResult<IndependentParticipationPacketV1> {
    check!((1..=16).contains(&index));
    let url = format!("https://www.mhsaa.com/sports/boys-track-field/stories/synthetic-count-fixture-{index}");
    let path = format!("/sports/boys-track-field/stories/synthetic-count-fixture-{index}");
    let body = format!("<!doctype html><html><head><link rel=\"canonical\" href=\"{url}\"></head><body><article class=\"node__article\" data-path=\"{path}\" data-nid=\"9900{index}\"><p class=\"publish-date\">May 15, 2026</p><h4><span class=\"text--secondary\">Zacchaeus Brocks ♦ Detroit Catholic Central | Senior ♦ Track</span></h4><p>SYNTHETIC COUNT TEST DOCUMENT {index}. No real publication or additional person authority is claimed.</p></article></body></html>");
    let time = DateTime::parse_from_rfc3339("2026-10-16T00:10:00Z")?.with_timezone(&Utc);
    let capture = RetainedHtmlCaptureV1::new(&url, &url, 200, time, body.as_bytes(), RetainedHttpProvenanceV1 {
        request_method: "GET".into(), request_headers: vec![("Accept".into(), "*/*".into())],
        response_header_bytes: b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nX-Synthetic-Test: source-count\r\n\r\n".to_vec(),
        content_type: "text/html; charset=utf-8".into(), body_encoding: RetainedBodyEncodingV1::Identity, started_at: time,
    })?;
    Ok(IndependentParticipationPacketV1::new(capture, schedule.clone())?)
}

pub(super) fn read(store: &Store, packet: &IndependentParticipationPacketV1) -> TestResult<(Vec<RetainedCaptureLeafV1>, RecordedIndependentParticipationV1)> {
    let snapshot = store.snapshot();
    let mut grant = Grant::new();
    let recorded = read_recorded_independent_participation(&snapshot, &packet.reference()?, &mut grant)?.ok_or("recorded synthetic packet")?;
    let root = recorded.root();
    let raw = snapshot.journal_payload_raw_bounded(root.schema.phase(), &root.key, JournalReadBudget::new(2 * 1024 * 1024)?)?.ok_or("retained packet payload")?;
    check!(eq; reference(raw.get().as_bytes())?, root.payload);
    let fields: BTreeMap<String, Box<RawValue>> = serde_json::from_str(raw.get())?;
    let status = fields.get("status").ok_or("retained status capture")?;
    let capture: RetainedHtmlCaptureV1 = serde_json::from_str(status.get())?;
    let body = capture.body()?;
    let mut leaves = Vec::new();
    for (kind, object) in [(RetainedCaptureLeafKindV1::CaptureJson, reference(status.get().as_bytes())?),
        (RetainedCaptureLeafKindV1::OriginalBody, reference(body.as_bytes())?)] {
        let leaf = RetainedCaptureLeafV1 { version: 1, root: root.clone(), slot: RetainedCaptureSlotV1::ParticipationStatus, kind, object };
        snapshot.verify_retained_capture_leaf_v1(&leaf, &mut grant)?;
        leaves.push(leaf);
    }
    Ok((leaves, recorded))
}

pub(super) fn schedule(store: &Store) -> TestResult<RetainedHtmlCaptureV1> {
    let snapshot = store.snapshot();
    let mut grant = Grant::new();
    let owner = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let metadata = owner.discover_active_subjects_v1(AttachmentSubjectRoleV1::AthleteStatus, &mut grant)?;
    let [item] = metadata.subjects() else { return Err("one original status before synthetic appends".into()); };
    let census_store::DiscoveredAttachmentCaptureV1::Retained(leaf) = item.capture() else { return Err("original retained status".into()); };
    snapshot.verify_retained_capture_leaf_v1(leaf, &mut grant)?;
    let raw = snapshot.journal_payload_raw_bounded(leaf.root.schema.phase(), &leaf.root.key, JournalReadBudget::new(2 * 1024 * 1024)?)?.ok_or("original packet")?;
    check!(eq; reference(raw.get().as_bytes())?, leaf.root.payload);
    let fields: BTreeMap<String, Box<RawValue>> = serde_json::from_str(raw.get())?;
    Ok(serde_json::from_str(fields.get("schedule").ok_or("original schedule")?.get())?)
}
