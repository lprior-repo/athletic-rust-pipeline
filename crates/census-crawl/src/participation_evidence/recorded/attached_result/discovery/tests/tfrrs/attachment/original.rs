use super::*;
use crate::participation_evidence::result_ownership::tfrrs::{
    CheckedRetainedTfrrsListV1, CheckedRetainedTfrrsRowV1,
};
use census_store::RetainedCaptureRootV1;

pub(super) struct Source {
    pub subject: RetainedDocumentSubjectReferenceV1,
    pub fact: AttachmentDigestV1,
    pub lineage: AttachmentDigestV1,
}

pub(super) fn read(
    objects: &mut Objects,
    checked: &CheckedRetainedTfrrsListV1<'_>,
    row: &CheckedRetainedTfrrsRowV1<'_>,
    root: &RetainedCaptureRootV1,
    case: Case,
) -> TestResult<Source> {
    let source = row.subject()?;
    let capture = objects.retained(RetainedCaptureLeafV1 {
        version: 1,
        root: root.clone(),
        slot: RetainedCaptureSlotV1::SelfCapture,
        kind: RetainedCaptureLeafKindV1::CaptureJson,
        object: root.payload.clone(),
    });
    let body = objects.retained(RetainedCaptureLeafV1 {
        version: 1,
        root: root.clone(),
        slot: RetainedCaptureSlotV1::SelfCapture,
        kind: RetainedCaptureLeafKindV1::OriginalBody,
        object: objects::reference(checked.original_document().as_bytes())?,
    });
    let mut interpretation = objects.json(&(
        "retained-tfrrs-list-row/v1",
        checked.capture().requested_url(),
        checked.capture().body_sha256(),
        row.locator(),
    ))?;
    check!(eq; interpretation.as_str(), row.reference());
    if matches!(case, Case::ChangedInterpretation) {
        interpretation =
            objects.json(&("controlled-invalid-row-interpretation", row.reference()))?;
    }
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
    check!(ne; context.as_str(), root.key);
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
    check!(eq; subject.as_str(), source.reference());
    let locator = locator(objects, row, source.locator(), case)?;
    let namespace = if matches!(case, Case::WrongNamespace) {
        "milesplit_result_document_v1"
    } else {
        source.document_namespace()
    };
    let fact = objects.bytes(row.original_row()?.as_bytes().to_vec())?;
    let lineage = objects.json(&("test-reviewed-original-native-link/v1", "tfrrs.org", root))?;
    Ok(Source {
        subject: RetainedDocumentSubjectReferenceV1 {
            namespace: text(namespace)?,
            native_id: Some(text(source.native_document_id())?),
            capture,
            body,
            interpretation,
            locator,
            subject,
            context,
            role: AttachmentSubjectRoleV1::ResultRow,
        },
        fact,
        lineage,
    })
}

fn locator(
    objects: &mut Objects,
    row: &CheckedRetainedTfrrsRowV1<'_>,
    original: &str,
    case: Case,
) -> TestResult<AttachmentDigestV1> {
    let value = if matches!(case, Case::ChangedLocator) {
        let start = row
            .locator()
            .byte_offset
            .checked_add(1)
            .ok_or("controlled locator overflow")?;
        let end = row
            .locator()
            .byte_offset
            .checked_add(row.locator().byte_length)
            .ok_or("controlled locator overflow")?;
        format!("html_bytes:{start}:{end}")
    } else {
        original.to_owned()
    };
    objects.bytes(value.into_bytes())
}
