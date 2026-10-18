use super::*;
fn reviewed(
    inspection: &EvidenceSubjectAttachmentInspectionV1<'_>,
    objects: &mut Objects,
    rows: &Rows,
    independent: &target::TargetMaterial,
) -> TestResult<ReviewedAttachmentContextV1> {
    let lineage = objects.json(&(
        "reviewed-publisher-family/v1",
        "milesplit.com",
        "controlled-three-marks-results",
    ))?;
    let mut facts = Vec::new();
    facts.try_reserve_exact(
        rows.facts
            .len()
            .checked_add(1)
            .ok_or("fact count overflow")?,
    )?;
    let mut pairs = Vec::new();
    pairs.try_reserve_exact(rows.facts.len())?;
    for (fact, subject) in &rows.facts {
        facts.push(ReviewedAttachmentFactV1 { fact: fact.clone(), subject: subject.clone(), publisher_lineage: lineage.clone(), disposition: AttachmentFactDispositionV1::SupportsAttachment, rationale: text("Controlled original result row explicitly reviewed against the independent named student context")? });
        pairs.push(AttachmentCorroborationPairV1 {
            subject_fact: fact.clone(),
            accepted_target_fact: independent.quote.clone(),
            basis: AttachmentCorroborationBasisV1::IndependentPersonSpecificContext,
            explanation: text(
                "Controlled explicit person-specific review; no automatic name-match authority",
            )?,
        });
    }
    facts.push(ReviewedAttachmentFactV1 { fact: independent.quote.clone(), subject: independent.target_subject.clone(), publisher_lineage: independent.lineage.clone(), disposition: AttachmentFactDispositionV1::SupportsAttachment, rationale: text("Original uniquely owned MHSAA named student heading reviewed against the actual source-bound target")? });
    facts.sort_by(|left, right| left.fact.cmp(&right.fact));
    let scope = objects.json(&(
        "controlled-three-marks-attachment-scope/v1",
        &rows.subjects,
        &facts,
        &pairs,
        &independent.target_subject,
        inspection.target(),
        inspection.accepted_target_witness(),
    ))?;
    let mut references = objects.references()?;
    references.extend(inspection.issued_objects().cloned());
    references.sort();
    references.dedup();
    Ok(ReviewedAttachmentContextV1 {
        objects: list(references)?,
        facts: list(facts)?,
        corroboration: list(pairs)?,
        complete_scope_records: list(vec![scope])?,
        accepted_target_subjects: list(vec![independent.target_subject.clone()])?,
        accepted_target_witness: inspection.accepted_target_witness().clone(),
    })
}
pub(super) fn commit(
    installed: &InstalledSource,
    inspection: EvidenceSubjectAttachmentInspectionV1<'_>,
    objects: &mut Objects,
    mut rows: Rows,
    independent: target::TargetMaterial,
) -> TestResult {
    rows.subjects
        .sort_by(|left, right| left.subject.cmp(&right.subject));
    let reviewed_context = reviewed(&inspection, objects, &rows, &independent)?;
    let command = RetainedEvidenceSubjectAttachmentCommandV1 {
        version: 1, policy: text(EVIDENCE_SUBJECT_ATTACHMENT_POLICY_V1)?, store: inspection.store_identity().clone(), operation: text("controlled-result-attachment")?, expected_public_head: inspection.public_head().clone(),
        expected_evidence_generation: inspection.evidence_generation(), expected_derived_generation: inspection.derived_generation(), expected_attachment_inventory: inspection.inventory().clone(), target: inspection.target().clone(), subjects: list(rows.subjects)?, action: inspection.action().clone(), reviewed_context,
        actor: AttachmentReviewActorV1::LocalOperator, reviewed_at: DateTime::parse_from_rfc3339("2026-10-17T00:00:00Z")?.with_timezone(&Utc), rationale: text("Synthetic controlled review of three original source rows; not authenticated human or genuine-source acceptance")?,
    };
    if !matches!(
        command.action,
        AttachmentActionV1::Attach {
            prior: AttachmentPredecessorV1::First
        }
    ) {
        return Err("controlled attachment predecessor differs".into());
    }
    let context = objects.borrowed_new(&installed.store)?;
    let prepared = installed.store.prepare_evidence_subject_attachment_v1(
        command,
        EvidenceSubjectAttachmentPreparationV1::New {
            inspection,
            context: &context,
        },
    )?;
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    match installed
        .store
        .apply_evidence_subject_attachment_v1(prepared, &approved, &digest)?
    {
        Application::Written(_) => Ok(()),
        Application::Repeated(_) => Err("controlled row attachment unexpectedly replayed".into()),
    }
}
