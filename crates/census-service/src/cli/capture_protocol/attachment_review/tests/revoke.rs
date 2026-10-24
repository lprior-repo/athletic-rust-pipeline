use super::context::{list, text};
use super::support::*;
use census_domain::evidence_subject_attachment::{
    replay_reviewed_references, AttachmentActionV1, AttachmentReceiptBindingV1,
    RetainedEvidenceSubjectAttachmentCommandV1,
};
use census_store::{Application, AttachmentInspectionRequestV1, Receipt, Store};
use serde_json::Value;

fn revoke_command(case: &Case) -> TestResult<RetainedEvidenceSubjectAttachmentCommandV1> {
    let installed = state(&case.root)?;
    let original = installed
        .records
        .first()
        .ok_or("original attachment absent")?;
    let (store, _writer) = Store::open_current_qualification_v3(&case.root)?.into_parts();
    let inspection =
        store.inspect_evidence_subject_attachment_v1(AttachmentInspectionRequestV1::Revoke {
            prior: original.reference(),
        })?;
    let mut command = original.command.clone();
    command.operation = text("controlled-cli-attachment-revocation")?;
    command.action = inspection.action().clone();
    command.target = inspection.target().clone();
    command.expected_public_head = inspection.public_head().clone();
    command.expected_attachment_inventory = inspection.inventory().clone();
    command.expected_evidence_generation = inspection.evidence_generation();
    command.expected_derived_generation = inspection.derived_generation();
    command.reviewed_context.accepted_target_witness = inspection.accepted_target_witness().clone();
    let mut objects = command.reviewed_context.objects.as_slice().to_vec();
    objects.extend(inspection.issued_objects().cloned());
    objects.sort();
    objects.dedup();
    command.reviewed_context.objects = list(objects)?;
    command.rationale = text("Controlled synthetic attachment revocation adapter test; no genuine-person eligibility claim")?;
    check!(eq; command.action, AttachmentActionV1::Revoke { prior: original.reference() });
    Ok(command)
}

#[test]
fn normal_cli_revoke_previews_applies_and_cold_replays_the_exact_original_receipt() -> TestResult {
    let mut case = Case::new(false)?;
    let approval = sha256(&invoke(&case, "attachment-review", None, true)?);
    let attached: Value =
        serde_json::from_slice(&invoke(&case, "attachment-review", Some(&approval), true)?)?;
    let first_application: Application = serde_json::from_value(attached["application"].clone())?;
    let Application::Written(first_receipt) = first_application else {
        return Err("fixture attachment was not Written".into());
    };
    let before = state(&case.root)?;
    check!(eq; before.inventory.records, 1);
    check!(eq; before.receipts, vec![first_receipt.clone()]);
    case.command = revoke_command(&case)?;
    case.write_command(&case.command)?;
    case.inline.clear();
    case.retained.clear();
    let original_command_bytes = std::fs::read(&case.command_path)?;
    let expected = case.command.approval_bytes(4 * 1024 * 1024)?;
    let preview = invoke(&case, "attachment-review", None, false)?;
    check!(eq; preview, expected);
    check!(eq; state(&case.root)?, before);
    check!(eq; std::fs::read(&case.command_path)?, original_command_bytes);
    let approval = sha256(&preview);
    let domain = case.command.approval_digest()?;
    check!(ne; approval.as_str(), domain.as_str());
    let applied: Value =
        serde_json::from_slice(&invoke(&case, "attachment-review", Some(&approval), false)?)?;
    check!(eq; applied["schema"], "attachment-review-v1");
    check!(eq; applied["operation"], case.command.operation.as_str());
    check!(eq; applied["approval_sha256"], approval);
    check!(eq; applied["approval_digest"], domain.as_str());
    check!(eq; applied["repeated"], false);
    let written: Application = serde_json::from_value(applied["application"].clone())?;
    let Application::Written(receipt) = written else {
        return Err("approved revocation did not produce Written".into());
    };
    let reply_receipt: Receipt = serde_json::from_value(applied["receipt"].clone())?;
    check!(eq; reply_receipt, receipt);
    let after = state(&case.root)?;
    check!(eq; after.inventory.records, 2);
    check!(eq; after.receipt_count, before.receipt_count.checked_add(1).ok_or("receipt count overflow")?);
    check!(eq; &after.head, &before.head);
    check!(eq; &after.tables, &before.tables);
    check!(eq; &after.counts, &before.counts);
    check!(eq; after.records.first(), before.records.first());
    check!(eq; after.records.last().ok_or("revocation absent")?.command, case.command);
    check!(eq; after.receipts, vec![first_receipt, receipt.clone()]);
    let bindings = after
        .records
        .iter()
        .map(|record| AttachmentReceiptBindingV1 {
            operation: record.command.operation.clone(),
            record_digest: record.record_digest.clone(),
        })
        .collect::<Vec<_>>();
    let projection = replay_reviewed_references(
        &case.command.store,
        &after.records,
        &bindings,
        &after.inventory,
    )?;
    check!(eq; projection.active_records().count(), 0);
    let replay: Value =
        serde_json::from_slice(&invoke(&case, "attachment-replay", Some(&approval), false)?)?;
    let repeated: Application = serde_json::from_value(replay["application"].clone())?;
    check!(eq; repeated, Application::Repeated(receipt));
    check!(eq; replay["repeated"], true);
    check!(eq; replay["appended"], 0);
    check!(eq; replay["approval_sha256"], approval);
    check!(eq; serde_json::to_vec(&replay["receipt"])?, serde_json::to_vec(&applied["receipt"])?);
    check!(eq; state(&case.root)?, after);
    check!(eq; std::fs::read(&case.command_path)?, original_command_bytes);
    Ok(())
}
