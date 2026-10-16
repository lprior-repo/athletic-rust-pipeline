use super::*;

pub(super) fn chain(
    store: &Store,
    sources: &Sources,
    target: &IdentityCommitOutcomeV1,
) -> TestResult<Vec<(RetainedEvidenceSubjectAttachmentCommandV1, Receipt)>> {
    let empty = History::read(store)?;
    check!(empty.records.is_empty());
    let first = stage("A", Stage::Article, store, sources, target, &empty)?;
    let history = History::read(store)?;
    history.require_active_article(store)?;
    check!(eq; history.records.len(), 1);
    check!(eq; history.records.first().ok_or("active A record")?.command, first.0);
    check!(eq; history.receipts.as_slice(), std::slice::from_ref(&first.1));
    report::phase(
        "A",
        "held-read",
        "active-original-article",
        json!({"inventory":history.inventory,
        "records":history.records, "receipts":history.receipts}),
    )?;
    let second = stage("B", Stage::Results, store, sources, target, &history)?;
    let retained = History::read(store)?;
    check!(eq; retained.records.len(), 2);
    check!(eq; retained.receipts.len(), 2);
    let second_record = retained.records.get(1).ok_or("B retained record")?;
    check!(eq; second_record.command, second.0);
    check!(eq; second_record.command.subjects.len(), 3);
    check!(eq; retained.receipts.get(1), Some(&second.1));
    let bindings = retained
        .records
        .iter()
        .map(|record| AttachmentReceiptBindingV1 {
            operation: record.command.operation.clone(),
            record_digest: record.record_digest.clone(),
        })
        .collect::<Vec<_>>();
    let projected = replay_reviewed_references(
        &store.snapshot().store_identity()?,
        &retained.records,
        &bindings,
        &retained.inventory,
    )?;
    check!(eq; projected.active_records().map(|record| record.command.operation.as_str()).collect::<Vec<_>>(),
        vec![ARTICLE_OPERATION, RESULTS_OPERATION]);
    report::phase(
        "B",
        "held-read",
        "both-attachments-active",
        json!({"inventory":retained.inventory,
        "records":retained.records, "receipts":retained.receipts}),
    )?;
    Ok(vec![first, second])
}

fn stage(
    label: &str,
    kind: Stage,
    store: &Store,
    sources: &Sources,
    target: &IdentityCommitOutcomeV1,
    history: &History,
) -> TestResult<(RetainedEvidenceSubjectAttachmentCommandV1, Receipt)> {
    let before = preservation::Held::capture(store, sources)?;
    let inspection =
        match store.inspect_evidence_subject_attachment_v1(AttachmentInspectionRequestV1::Attach {
            target: target.public_id().clone(),
            prior: AttachmentPredecessorV1::First,
        }) {
            Ok(inspection) => inspection,
            Err(error) => return refuse(label, "inspect", error, store, sources, &before),
        };
    report::phase(
        label,
        "inspect",
        "issued",
        json!({"inventory":inspection.inventory(),
        "target":inspection.target(), "witness":inspection.accepted_target_witness(),
        "issued_objects":inspection.issued_objects().collect::<Vec<_>>()}),
    )?;
    let context = match Context::build(kind, sources, store, &inspection, history) {
        Ok(context) => context,
        Err(error) => {
            before.require_unchanged(store, sources)?;
            report::phase(
                label,
                "construct",
                "refused-before-command",
                json!({"typed_error":format!("{error:?}"),
                "state":"Exact source/attachment namespace fingerprints, logical payloads and all receipts unchanged"}),
            )?;
            return Err(error);
        }
    };
    let expected_subjects = match kind {
        Stage::Article => 1,
        Stage::Results => 3,
    };
    let expected_pairs = match kind {
        Stage::Article => 2,
        Stage::Results => 3,
    };
    check!(eq; context.command.subjects.len(), expected_subjects);
    check!(eq; context.command.reviewed_context.corroboration.len(), expected_pairs);
    report::emit(label, sources, store, &context)?;
    let command = context.command.clone();
    let borrowed = context.borrowed();
    let prepared = match store.prepare_evidence_subject_attachment_v1(
        command.clone(),
        EvidenceSubjectAttachmentPreparationV1::New {
            inspection,
            context: &borrowed,
        },
    ) {
        Ok(prepared) => prepared,
        Err(error) => return refuse(label, "prepare", error, store, sources, &before),
    };
    check!(!prepared.is_replay());
    check!(eq; prepared.command(), &command);
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    report::phase(
        label,
        "prepare",
        "prepared-for-controlled-test-decision",
        json!({
        "approval_sha256":hash(&approval), "approval_digest":digest, "approval_bytes":approval.len(),
        "decision":"Controlled explicit test approval only; no authenticated human identity approval"}),
    )?;
    let result = store.apply_evidence_subject_attachment_v1(prepared, &approval, &digest);
    let receipt = match result {
        Ok(Application::Written(receipt)) => receipt,
        Ok(Application::Repeated(_)) => {
            return Err("new genuine attachment unexpectedly replayed".into())
        }
        Err(error) => return refuse(label, "apply", error, store, sources, &before),
    };
    check!(eq; store.receipt(&receipt.operation)?, Some(receipt.clone()));
    require_originals(store, sources)?;
    report::phase(
        label,
        "apply",
        "written",
        json!({"receipt":receipt,
        "physical_inventory":report::physical_inventory(store, sources)?}),
    )?;
    Ok((command, receipt))
}

fn refuse<T>(
    stage: &str,
    phase: &str,
    error: StoreError,
    store: &Store,
    sources: &Sources,
    before: &preservation::Held<'_>,
) -> TestResult<T> {
    before.require_unchanged(store, sources)?;
    require_originals(store, sources)?;
    let detail = match &error {
        StoreError::EvidenceSubjectAttachment(EvidenceSubjectAttachmentStoreError::Domain(
            EvidenceSubjectAttachmentError::Capacity {
                resource,
                needed,
                available,
            },
        )) => {
            check!(needed > available);
            json!({"typed":"EvidenceSubjectAttachment::Domain::Capacity", "resource":resource,
                    "needed":needed, "available":available, "state":"protected inputs/roots/receipts unchanged",
                    "disposition":"Delivery blocker; evidence is not omitted and grant is not reset"})
        }
        other => {
            json!({"typed_error":format!("{other:?}"), "state":"protected inputs/roots/receipts unchanged"})
        }
    };
    report::phase(stage, phase, "refused", detail)?;
    Err(error.into())
}

pub(super) fn require_originals(store: &Store, sources: &Sources) -> TestResult {
    let snapshot = store.snapshot();
    for root in &sources.roots {
        let raw = snapshot
            .journal_payload_raw_bounded(
                root.schema.phase(),
                &root.key,
                JournalReadBudget::new(2 * 1024 * 1024)?,
            )?
            .ok_or("complete original root after workflow")?;
        check!(eq; reference(raw.get().as_bytes())?, root.payload);
    }
    for capture in &sources.captures {
        check!(eq; reference(&capture.original_json)?, capture.json);
        check!(eq; reference(&capture.original_body)?, capture.body);
        check!(eq; capture.capture.body()?.as_bytes(), capture.original_body.as_slice());
    }
    Ok(())
}
