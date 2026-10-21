use super::*;

pub(super) fn attach(
    installed: &source::Installed,
    status: bool,
) -> TestResult<(
    RetainedEvidenceSubjectAttachmentCommandV1,
    census_store::Receipt,
)> {
    attach_named(installed, status, None)
}

pub(super) fn attach_named(
    installed: &source::Installed,
    status: bool,
    operation: Option<&str>,
) -> TestResult<(
    RetainedEvidenceSubjectAttachmentCommandV1,
    census_store::Receipt,
)> {
    let inspection = installed.store.inspect_evidence_subject_attachment_v1(
        AttachmentInspectionRequestV1::Attach {
            target: installed.public.clone(),
            prior: AttachmentPredecessorV1::First,
        },
    )?;
    check!(eq; inspection.target().canonical_candidate, installed.athlete.id.cast());
    let mut objects = Objects::default();
    let source = if status {
        material::status(installed, &mut objects)?
    } else {
        material::result(installed, &mut objects)?
    };
    let reviewed_context = context(&inspection, &mut objects, &source)?;
    let command = command(
        &inspection,
        status,
        operation,
        source.subject,
        reviewed_context,
    )?;
    let supplied = objects.borrowed_new(&installed.store)?;
    let prepared = installed.store.prepare_evidence_subject_attachment_v1(
        command.clone(),
        EvidenceSubjectAttachmentPreparationV1::New {
            inspection,
            context: &supplied,
        },
    )?;
    check!(!prepared.is_replay());
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    let Application::Written(receipt) = installed
        .store
        .apply_evidence_subject_attachment_v1(prepared, &approval, &digest)?
    else {
        return Err("first actual Tessa source attachment unexpectedly replayed".into());
    };
    Ok((command, receipt))
}

fn context(
    inspection: &EvidenceSubjectAttachmentInspectionV1<'_>,
    objects: &mut Objects,
    source: &material::Material,
) -> TestResult<ReviewedAttachmentContextV1> {
    let target = objects.json(&(
        "genuine-tessa-reviewed-native-target/v1",
        inspection.target(),
        inspection.accepted_target_witness(),
        &source.target_fact,
        &source.target_lineage,
        &source.independent_context,
    ))?;
    let mut facts = vec![
        ReviewedAttachmentFactV1 { fact: source.fact.clone(), subject: source.subject.subject.clone(), publisher_lineage: source.lineage.clone(), disposition: AttachmentFactDispositionV1::SupportsAttachment, rationale: text("Exact original source fact retained; selected school event-day does not establish a tenure interval")? },
        ReviewedAttachmentFactV1 { fact: source.target_fact.clone(), subject: target.clone(), publisher_lineage: source.target_lineage.clone(), disposition: AttachmentFactDispositionV1::SupportsAttachment, rationale: text("Actual accepted MS13658157 roster row, checked independent Roncalli junior2026 and exactMarch11 original3200mark, matched to nativeTF8779607 through exact protected document attachment; no global same-person merge")? },
    ];
    facts.sort_by(|left, right| left.fact.cmp(&right.fact));
    let pairs = vec![AttachmentCorroborationPairV1 { subject_fact: source.fact.clone(), accepted_target_fact: source.target_fact.clone(), basis: source.basis, explanation: text("Controlled qualification review of literal native identity and independent Roncalli name, school, junior grade, March11 calendar and original mark; no production approval or inferred current membership")? }];
    let scope = objects.json(&(
        "genuine-tessa-reviewed-complete-scope/v1",
        &source.subject,
        &source.independent_context,
        &facts,
        &pairs,
        &target,
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
        accepted_target_subjects: list(vec![target])?,
        accepted_target_witness: inspection.accepted_target_witness().clone(),
    })
}

fn command(
    inspection: &EvidenceSubjectAttachmentInspectionV1<'_>,
    status: bool,
    operation: Option<&str>,
    subject: RetainedDocumentSubjectReferenceV1,
    reviewed_context: ReviewedAttachmentContextV1,
) -> TestResult<RetainedEvidenceSubjectAttachmentCommandV1> {
    let default_operation = if status {
        "genuine-tessa-roncalli-exact-day-test-attachment"
    } else {
        "genuine-tessa-native-tfrrs-test-attachment"
    };
    let operation = match operation {
        Some(value) => value,
        None => default_operation,
    };
    Ok(RetainedEvidenceSubjectAttachmentCommandV1 {
        version: 1,
        policy: text(EVIDENCE_SUBJECT_ATTACHMENT_POLICY_V1)?,
        store: inspection.store_identity().clone(),
        operation: text(operation)?,
        expected_public_head: inspection.public_head().clone(),
        expected_evidence_generation: inspection.evidence_generation(),
        expected_derived_generation: inspection.derived_generation(),
        expected_attachment_inventory: inspection.inventory().clone(),
        target: inspection.target().clone(),
        subjects: list(vec![subject])?,
        action: inspection.action().clone(),
        actor: AttachmentReviewActorV1::LocalOperator,
        reviewed_at: DateTime::parse_from_rfc3339("2026-10-21T22:00:00Z")?.to_utc(),
        rationale: text("Genuine retained originals with explicitly controlled qualification reviewer action, separate from production acquisition ownership and human approval")?,
        reviewed_context,
    })
}

pub(super) fn replay(
    store: &Store,
    command: RetainedEvidenceSubjectAttachmentCommandV1,
    expected: &census_store::Receipt,
) -> TestResult {
    let before = (
        store.snapshot().sequence(),
        store.receipt_count()?,
        store.snapshot().tables_digest(&Table::ALL)?,
    );
    let prepared = store.prepare_evidence_subject_attachment_v1(
        command,
        EvidenceSubjectAttachmentPreparationV1::ReplayOnly,
    )?;
    check!(prepared.is_replay());
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    check!(
        matches!(store.apply_evidence_subject_attachment_v1(prepared, &approval, &digest)?, Application::Repeated(receipt) if &receipt == expected)
    );
    check!(eq; (store.snapshot().sequence(), store.receipt_count()?, store.snapshot().tables_digest(&Table::ALL)?), before);
    Ok(())
}
