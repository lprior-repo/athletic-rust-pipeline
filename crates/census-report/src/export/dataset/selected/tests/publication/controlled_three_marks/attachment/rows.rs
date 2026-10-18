use super::*;
pub(super) fn push_row(
    snapshot: &StoreSnapshot<'_>,
    locator: &RowLocator,
    grant: &mut AttachmentReplayGrantV1,
    objects: &mut Objects,
    rows: &mut Rows,
) -> TestResult {
    let retained = read_recorded_html_result_capture(
        snapshot,
        &locator.document_key,
        &[locator.locator],
        grant,
    )?
    .ok_or("recorded controlled result absent")?;
    let document = retained.read(&[locator.locator])?;
    if document.context_reference() != locator.document_key {
        return Err("controlled result context differs".into());
    }
    let row = document.row(0).ok_or("controlled result row absent")?;
    if row.locator() != locator.locator {
        return Err("controlled original row locator differs".into());
    }
    match &rows.name {
        Some(name) if name != &row.parsed_row().name => {
            return Err("controlled row names differ".into())
        }
        None => rows.name = Some(row.parsed_row().name.clone()),
        Some(_) => {}
    }
    let capture = objects.retained(RetainedCaptureLeafV1 {
        version: 1,
        root: retained.root().clone(),
        slot: RetainedCaptureSlotV1::SelfCapture,
        kind: RetainedCaptureLeafKindV1::CaptureJson,
        object: retained.root().payload.clone(),
    })?;
    let body = objects.retained(RetainedCaptureLeafV1 {
        version: 1,
        root: retained.root().clone(),
        slot: RetainedCaptureSlotV1::SelfCapture,
        kind: RetainedCaptureLeafKindV1::OriginalBody,
        object: objects::reference(document.original_document().as_bytes())?,
    })?;
    let page = document.parsed_page();
    let context = objects.json(&(
        "retained-milesplit-html-result-row/v1",
        document.capture(),
        &page.meet,
        &page.skipped,
        &page.grade_issues,
        document.published_start(),
        document.published_end(),
    ))?;
    if context.as_str() != document.context_reference() {
        return Err("controlled result context digest differs".into());
    }
    let source = row.subject()?;
    let interpretation = objects.json(&(
        "retained-milesplit-html-result-row/v1",
        document.capture().requested_url(),
        document.capture().body_sha256(),
        row.locator(),
    ))?;
    if interpretation.as_str() != row.reference() {
        return Err("controlled row interpretation differs".into());
    }
    let subject = objects.bytes(
        format!(
            "document_subject_v1\n{}\n{}\n{}\n{}\n{}",
            source.document_namespace(),
            source.native_document_id(),
            source.document_url(),
            source.body_sha256(),
            source.locator()
        )
        .into_bytes(),
    )?;
    if subject.as_str() != source.reference() {
        return Err("controlled subject digest differs".into());
    }
    let locator = objects.bytes(source.locator().as_bytes().to_vec())?;
    let fact = objects.bytes(row.original_row()?.as_bytes().to_vec())?;
    rows.subjects.push(RetainedDocumentSubjectReferenceV1 {
        namespace: text(source.document_namespace())?,
        native_id: Some(text(source.native_document_id())?),
        capture,
        body,
        interpretation,
        locator,
        subject: subject.clone(),
        context,
        role: AttachmentSubjectRoleV1::ResultRow,
    });
    rows.facts.push((fact, subject));
    Ok(())
}
