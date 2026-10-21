use super::*;

pub(in super::super) fn read(
    installed: &source::Installed,
    objects: &mut objects::Objects,
) -> TestResult<Material> {
    let snapshot = installed.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let recorded = read_recorded_tfrrs_list_capture(
        &snapshot,
        &installed.context,
        &[source::locator()],
        &mut grant,
    )?
    .ok_or("recorded genuine Tessa list")?;
    let checked = recorded.read(&[source::locator()])?;
    let row = checked.row(0).ok_or("exact original Tessa result")?;
    let document = row.subject()?;
    let capture = objects.retained(RetainedCaptureLeafV1 {
        version: 1,
        root: recorded.root().clone(),
        slot: RetainedCaptureSlotV1::SelfCapture,
        kind: RetainedCaptureLeafKindV1::CaptureJson,
        object: recorded.root().payload.clone(),
    });
    let body = objects.retained(RetainedCaptureLeafV1 {
        version: 1,
        root: recorded.root().clone(),
        slot: RetainedCaptureSlotV1::SelfCapture,
        kind: RetainedCaptureLeafKindV1::OriginalBody,
        object: objects::reference(checked.original_document().as_bytes())?,
    });
    let interpretation = objects.json(&(
        "retained-tfrrs-list-row/v1",
        checked.capture().requested_url(),
        checked.capture().body_sha256(),
        row.locator(),
    ))?;
    check!(eq; interpretation.as_str(), row.reference());
    let context = objects.json(&(
        "retained-tfrrs-row-context/v1",
        checked.capture().requested_url(),
        checked.capture().body_sha256(),
        row.section_span(),
        row.event_header_span(),
        row.meet_id(),
        row.published_date(),
        row.event_label(),
        row.gender(),
    ))?;
    check!(eq; context.as_str(), row.context_reference());
    let subject_digest = subject(objects, &document)?;
    let locator = objects.bytes(document.locator().as_bytes().to_vec())?;
    let fact = objects.bytes(row.original_row()?.as_bytes().to_vec())?;
    let target_fact = objects.bytes(
        checked
            .original_document()
            .get(row.athlete_span())
            .ok_or("original native Tessa anchor")?
            .as_bytes()
            .to_vec(),
    )?;
    let lineage = objects.json(&("genuine-tessa-publisher/v1", "tfrrs.org", recorded.root()))?;
    Ok(Material {
        subject: RetainedDocumentSubjectReferenceV1 {
            namespace: text(document.document_namespace())?,
            native_id: Some(text(document.native_document_id())?),
            capture,
            body,
            interpretation,
            locator,
            subject: subject_digest,
            context,
            role: AttachmentSubjectRoleV1::ResultRow,
        },
        fact,
        target_fact,
        target_lineage: lineage.clone(),
        lineage,
        basis: AttachmentCorroborationBasisV1::ExplicitOriginalCrossReference,
    })
}
