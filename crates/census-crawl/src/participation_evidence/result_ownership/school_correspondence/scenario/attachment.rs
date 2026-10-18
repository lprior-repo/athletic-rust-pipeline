use super::*;
use census_store::{
    AttachmentContextObjectV1, AttachmentInspectionRequestV1,
    EvidenceSubjectAttachmentPreparationV1, RetainedCaptureLeafKindV1, RetainedCaptureLeafV1,
    RetainedCaptureSlotV1,
};

mod objects;
use objects::Objects;

pub(super) fn apply(store: &Store, input: &setup::Input) -> TestResult {
    let inspection =
        store.inspect_evidence_subject_attachment_v1(AttachmentInspectionRequestV1::Attach {
            target: input.public.clone(),
            prior: AttachmentPredecessorV1::First,
        })?;
    check!(eq; inspection.target().canonical_candidate, input.candidate);
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let retained = read_recorded_html_result_capture(
        &snapshot,
        &input.context,
        &setup::locators(),
        &mut grant,
    )?
    .ok_or("retained results")?;
    let document = retained.read(&setup::locators())?;
    let mut objects = Objects::default();
    let capture = objects.retained(RetainedCaptureLeafV1 {
        version: 1,
        root: retained.root().clone(),
        slot: RetainedCaptureSlotV1::SelfCapture,
        kind: RetainedCaptureLeafKindV1::CaptureJson,
        object: retained.root().payload.clone(),
    });
    let body = objects.retained(RetainedCaptureLeafV1 {
        version: 1,
        root: retained.root().clone(),
        slot: RetainedCaptureSlotV1::SelfCapture,
        kind: RetainedCaptureLeafKindV1::OriginalBody,
        object: objects::reference(document.original_document().as_bytes())?,
    });
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
    check!(eq; context.as_str(), document.context_reference());
    let milesplit = objects.json(&(
        "reviewed-publisher-family/v1",
        "milesplit.com",
        input.target.reference()?.as_str(),
        retained.root(),
    ))?;
    let independent = objects.json(&(
        "reviewed-publisher-family/v1",
        "mhsaa.com",
        input.article.requested_url(),
        input.article.body_sha256(),
    ))?;
    let article = input.article.body()?;
    check!(eq; article.as_bytes(), setup::ARTICLE);
    let paragraph = article
        .get(113861..114528)
        .ok_or("original MHSAA named senior/hurdles paragraph")?;
    check!(paragraph.contains("Brocks"));
    let original_article = objects.bytes(article.as_bytes().to_vec())?;
    let paragraph_fact = objects.bytes(paragraph.as_bytes().to_vec())?;
    let provider = input.target.read()?;
    let target = objects.json(&(
        "reviewed-original-person-specific-target/v1",
        inspection.accepted_target_witness(),
        provider.profile().person(),
        provider.profile().name(),
        input.article.requested_url(),
        &original_article,
        &paragraph_fact,
    ))?;
    let provider_packet = objects.bytes(input.target.encode()?)?;
    let (mut subjects, mut facts, pairs) = rows(
        &document,
        &mut objects,
        &capture,
        &body,
        &context,
        &milesplit,
        &paragraph_fact,
    )?;
    subjects.sort_by(|left, right| left.subject.cmp(&right.subject));
    facts.push(ReviewedAttachmentFactV1 { fact: paragraph_fact.clone(), subject: target.clone(), publisher_lineage: independent,
        disposition: AttachmentFactDispositionV1::SupportsAttachment, rationale: text("Original MHSAA named senior dual-hurdles context, explicitly reviewed against the actual source-bound target")? });
    facts.sort_by(|left, right| left.fact.cmp(&right.fact));
    let scope = objects.json(&(
        "reviewed-complete-school-row-attachment-scope/v1",
        retained.root(),
        &subjects,
        &facts,
        &pairs,
        &original_article,
        &provider_packet,
        inspection.target(),
        inspection.accepted_target_witness(),
    ))?;
    let mut references = objects.references();
    references.extend(inspection.issued_objects().cloned());
    references.sort();
    references.dedup();
    let command = RetainedEvidenceSubjectAttachmentCommandV1 {
        version: 1, policy: text(EVIDENCE_SUBJECT_ATTACHMENT_POLICY_V1)?, store: inspection.store_identity().clone(),
        operation: text("school-correspondence-public-person-attachment")?, expected_public_head: inspection.public_head().clone(),
        expected_evidence_generation: inspection.evidence_generation(), expected_derived_generation: inspection.derived_generation(),
        expected_attachment_inventory: inspection.inventory().clone(), target: inspection.target().clone(), subjects: list(subjects)?,
        action: inspection.action().clone(), reviewed_context: ReviewedAttachmentContextV1 {
            objects: list(references)?, facts: list(facts)?, corroboration: list(pairs)?, complete_scope_records: list(vec![scope])?,
            accepted_target_subjects: list(vec![target])?, accepted_target_witness: inspection.accepted_target_witness().clone(),
        }, actor: AttachmentReviewActorV1::LocalOperator, reviewed_at: at()?,
        rationale: text("Explicit protected review of all three original rows against the retained independent named senior/hurdles paragraph and actual provider-born target; no accepted facts injected")?,
    };
    let context = objects.borrowed();
    let prepared = store.prepare_evidence_subject_attachment_v1(
        command,
        EvidenceSubjectAttachmentPreparationV1::New {
            inspection,
            context: &context,
        },
    )?;
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    check!(matches!(
        store.apply_evidence_subject_attachment_v1(prepared, &approved, &digest)?,
        Application::Written(_)
    ));
    Ok(())
}

fn rows(
    document: &crate::participation_evidence::result_ownership::CheckedRetainedResultDocumentV1<'_>,
    objects: &mut Objects,
    capture: &AttachmentDigestV1,
    body: &AttachmentDigestV1,
    context: &AttachmentDigestV1,
    lineage: &AttachmentDigestV1,
    target_fact: &AttachmentDigestV1,
) -> TestResult<(
    Vec<RetainedDocumentSubjectReferenceV1>,
    Vec<ReviewedAttachmentFactV1>,
    Vec<AttachmentCorroborationPairV1>,
)> {
    let mut subjects = Vec::new();
    let mut facts = Vec::new();
    let mut pairs = Vec::new();
    for index in 0..document.selected_count() {
        let row = document.row(index).ok_or("original selected result")?;
        check!(eq; row.parsed_row().name, "Zacchaeus Brocks");
        let source = row.subject()?;
        let interpretation = objects.json(&(
            "retained-milesplit-html-result-row/v1",
            document.capture().requested_url(),
            document.capture().body_sha256(),
            row.locator(),
        ))?;
        check!(eq; interpretation.as_str(), row.reference());
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
        let locator = objects.bytes(source.locator().as_bytes().to_vec())?;
        let fact = objects.bytes(row.original_row()?.as_bytes().to_vec())?;
        subjects.push(RetainedDocumentSubjectReferenceV1 {
            namespace: text(source.document_namespace())?,
            native_id: Some(text(source.native_document_id())?),
            capture: capture.clone(),
            body: body.clone(),
            interpretation,
            locator,
            subject: subject.clone(),
            context: context.clone(),
            role: AttachmentSubjectRoleV1::ResultRow,
        });
        facts.push(ReviewedAttachmentFactV1 { fact: fact.clone(), subject, publisher_lineage: lineage.clone(), disposition: AttachmentFactDispositionV1::SupportsAttachment,
            rationale: text("Actual original name, grade12, school, event and mark retained; different meet marks are not claimed as direct cross-reference")? });
        pairs.push(AttachmentCorroborationPairV1 { subject_fact: fact, accepted_target_fact: target_fact.clone(),
            basis: AttachmentCorroborationBasisV1::IndependentPersonSpecificContext,
            explanation: text("Explicit person-specific review of original named senior dual-hurdles context and source-bound provider target")? });
    }
    check!(eq; subjects.len(), 3);
    Ok((subjects, facts, pairs))
}
