use super::objects::{self, Objects};
use super::source::TestResult;
use base64::Engine as _;
use census_crawl::participation_evidence::recorded::{
    read_recorded_independent_participation, read_recorded_independent_participation_window,
};
use census_crawl::participation_evidence::{
    DocumentSubjectV1, ParticipationEvidenceRefV1, SubjectOwnershipV1,
};
use census_domain::evidence_subject_attachment::{
    AttachmentDigestV1, AttachmentObjectReferenceV1, AttachmentReplayGrantV1,
    AttachmentSubjectRoleV1, AttachmentTextV1, RetainedDocumentSubjectReferenceV1,
};
use census_store::{
    JournalReadBudget, RetainedCaptureLeafKindV1, RetainedCaptureLeafV1, RetainedCaptureRootV1,
    RetainedCaptureSlotV1, StoreSnapshot,
};
use serde::Deserialize;
use serde_json::value::RawValue;

mod point;
mod window;

pub(super) enum Kind {
    Point,
    Window,
}

pub(super) struct SourceMaterial {
    pub(super) subject: RetainedDocumentSubjectReferenceV1,
    pub(super) fact: AttachmentDigestV1,
    pub(super) lineage: AttachmentDigestV1,
}

#[derive(Deserialize)]
struct PointSlots<'a> {
    #[serde(borrow)]
    status: &'a RawValue,
}

#[derive(Deserialize)]
struct WindowSlots<'a> {
    #[serde(borrow)]
    window: &'a RawValue,
}

#[derive(Deserialize)]
struct CaptureView<'a> {
    #[serde(borrow)]
    requested_url: &'a str,
    body_bytes: usize,
    #[serde(borrow)]
    body_sha256: &'a str,
    #[serde(borrow)]
    body_base64: &'a str,
}

pub(super) fn read(
    snapshot: &StoreSnapshot<'_>,
    reference: &ParticipationEvidenceRefV1,
    kind: Kind,
    objects: &mut Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult<SourceMaterial> {
    match kind {
        Kind::Point => point::read(snapshot, reference, objects, grant),
        Kind::Window => window::read(snapshot, reference, objects, grant),
    }
}

fn text(value: &str) -> TestResult<AttachmentTextV1> {
    Ok(AttachmentTextV1::parse(value)?)
}

fn raw_for(
    snapshot: &StoreSnapshot<'_>,
    root: &RetainedCaptureRootV1,
) -> TestResult<Box<RawValue>> {
    let budget = JournalReadBudget::new(2 * 1024 * 1024)?;
    let raw = snapshot
        .journal_payload_raw_bounded(root.schema.phase(), &root.key, budget)?
        .ok_or("raw packet missing")?;
    let actual = objects::reference(raw.get().as_bytes())?;
    if actual != root.payload {
        return Err("raw packet differs".into());
    }
    Ok(raw)
}

fn body_for(view: &CaptureView<'_>) -> TestResult<Vec<u8>> {
    if view.body_bytes == 0 || view.body_bytes > 512 * 1024 {
        return Err("body bounds".into());
    }
    let body = base64::engine::general_purpose::STANDARD.decode(view.body_base64)?;
    if body.len() != view.body_bytes {
        return Err("body length".into());
    }
    Ok(body)
}

fn quote_for<'a>(locator: &str, body: &'a [u8]) -> TestResult<&'a [u8]> {
    let rest = locator
        .strip_prefix("html_bytes:")
        .ok_or("locator prefix")?;
    let (start_text, end_text) = rest.split_once(':').ok_or("locator span")?;
    let start: usize = start_text.parse().map_err(|_| "locator start")?;
    let end: usize = end_text.parse().map_err(|_| "locator end")?;
    if start > end || end > body.len() {
        return Err("locator range".into());
    }
    body.get(start..end).ok_or("locator slice".into())
}

fn verified_bytes(
    objects: &mut Objects,
    preimage: Vec<u8>,
    expected: &str,
) -> TestResult<AttachmentDigestV1> {
    let digest = objects.bytes(preimage)?;
    if digest.as_str() != expected {
        return Err("preimage differs".into());
    }
    Ok(digest)
}

fn document_only<'a>(
    ownership: &'a SubjectOwnershipV1,
    subject: &'a DocumentSubjectV1,
) -> TestResult<&'a DocumentSubjectV1> {
    let document = match ownership {
        SubjectOwnershipV1::DocumentOnly(document) => document,
        SubjectOwnershipV1::ProviderLink(_) => {
            return Err("requires documentary ownership".into());
        }
    };
    if document != subject {
        return Err("subject differs".into());
    }
    Ok(document)
}

fn leaves(
    objects: &mut Objects,
    root: &RetainedCaptureRootV1,
    slot: RetainedCaptureSlotV1,
    kind: RetainedCaptureLeafKindV1,
    capture_object: AttachmentObjectReferenceV1,
    body_object: AttachmentObjectReferenceV1,
) -> TestResult<(AttachmentDigestV1, AttachmentDigestV1)> {
    let capture = objects.retained(RetainedCaptureLeafV1 {
        version: 1,
        root: root.clone(),
        slot,
        kind,
        object: capture_object,
    })?;
    let body = objects.retained(RetainedCaptureLeafV1 {
        version: 1,
        root: root.clone(),
        slot,
        kind: RetainedCaptureLeafKindV1::OriginalBody,
        object: body_object,
    })?;
    Ok((capture, body))
}

fn preimages(
    objects: &mut Objects,
    document: &DocumentSubjectV1,
    proof_capture: &str,
    proof_locator: &str,
    proof_interpretation: &str,
    tag: &str,
) -> TestResult<(AttachmentDigestV1, AttachmentDigestV1, AttachmentDigestV1)> {
    let interpretation = verified_bytes(
        objects,
        format!("{tag}\n{proof_capture}\n{proof_locator}").into_bytes(),
        proof_interpretation,
    )?;
    let subject = verified_bytes(
        objects,
        format!(
            "document_subject_v1\n{}\n{}\n{}\n{}\n{}",
            document.document_namespace(),
            document.native_document_id(),
            document.document_url(),
            document.body_sha256(),
            document.locator()
        )
        .into_bytes(),
        &document.reference(),
    )?;
    let locator = objects.bytes(proof_locator.as_bytes().to_vec())?;
    Ok((interpretation, subject, locator))
}

fn subject_ref(
    document: &DocumentSubjectV1,
    capture: AttachmentDigestV1,
    body: AttachmentDigestV1,
    interpretation: AttachmentDigestV1,
    locator: AttachmentDigestV1,
    subject: AttachmentDigestV1,
    context: AttachmentDigestV1,
) -> TestResult<RetainedDocumentSubjectReferenceV1> {
    Ok(RetainedDocumentSubjectReferenceV1 {
        namespace: text(document.document_namespace())?,
        native_id: Some(text(document.native_document_id())?),
        capture,
        body,
        interpretation,
        locator,
        subject,
        context,
        role: AttachmentSubjectRoleV1::AthleteStatus,
    })
}
