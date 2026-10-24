use self::support::*;
use census_store::{Application, Receipt};
use serde_json::Value;

mod context;
mod refusals;
mod revoke;
pub(super) mod support;

#[test]
fn preview_wrong_approval_apply_and_cold_original_replay_preserve_the_exact_contract() -> TestResult
{
    let case = Case::new(false)?;
    let before = state(&case.root)?;
    let original_command = std::fs::read(&case.command_path)?;
    let expected = case.command.approval_bytes(4 * 1024 * 1024)?;
    let preview = invoke(&case, "attachment-review", None, true)?;
    check!(eq; preview, expected);
    check!(eq; state(&case.root)?, before);
    let approved = sha256(&preview);
    let domain_digest = case.command.approval_digest()?;
    check!(ne; approved.as_str(), domain_digest.as_str());

    for wrong in ["0".repeat(64), domain_digest.as_str().to_owned()] {
        let error = invoke(&case, "attachment-review", Some(&wrong), true)
            .err()
            .ok_or("wrong approval was accepted")?;
        check!(error.to_string().contains("approval"));
        check!(eq; state(&case.root)?, before);
    }

    let output = invoke(&case, "attachment-review", Some(&approved), true)?;
    let reply: Value = serde_json::from_slice(&output)?;
    check!(eq; reply["schema"], "attachment-review-v1");
    check!(eq; reply["operation"], case.command.operation.as_str());
    check!(eq; reply["approval_sha256"], approved);
    check!(eq; reply["approval_digest"], domain_digest.as_str());
    check!(eq; reply["repeated"], false);
    let first: Application = serde_json::from_value(reply["application"].clone())?;
    let Application::Written(receipt) = first else {
        return Err("first approved attachment did not produce Written".into());
    };
    let reply_receipt: Receipt = serde_json::from_value(reply["receipt"].clone())?;
    check!(eq; reply_receipt, receipt);
    let after = state(&case.root)?;
    check!(eq; &after.head, &before.head);
    check!(eq; &after.tables, &before.tables);
    check!(eq; &after.counts, &before.counts);
    check!(eq; after.inventory.records, 1);
    check!(eq; after.receipt_count, before.receipt_count + 1);
    check!(eq; after.records.len(), 1);
    check!(eq; after.records.first().ok_or("attachment record")?.command, case.command);
    check!(eq; after.receipts, vec![receipt.clone()]);
    check!(eq; std::fs::read(&case.command_path)?, original_command);

    let replay: Value =
        serde_json::from_slice(&invoke(&case, "attachment-replay", Some(&approved), false)?)?;
    check!(eq; replay["schema"], "attachment-review-v1");
    check!(eq; replay["approval_sha256"], approved);
    check!(eq; replay["approval_digest"], domain_digest.as_str());
    check!(eq; replay["repeated"], true);
    check!(eq; replay["appended"], 0);
    let repeated: Application = serde_json::from_value(replay["application"].clone())?;
    check!(eq; repeated, Application::Repeated(receipt.clone()));
    check!(eq; serde_json::to_vec(&replay["receipt"])?, serde_json::to_vec(&reply["receipt"])?);
    check!(eq; state(&case.root)?, after);
    check!(eq; std::fs::read(&case.command_path)?, original_command);
    Ok(())
}

#[test]
fn nonempty_retained_original_body_is_verified_applied_and_replayed_through_normal_cli(
) -> TestResult {
    let case = Case::new(true)?;
    check!(eq; case.retained.len(), 1);
    let before = state(&case.root)?;
    let preview = invoke(&case, "attachment-review", None, true)?;
    check!(eq; preview, case.command.approval_bytes(4 * 1024 * 1024)?);
    check!(eq; state(&case.root)?, before);
    let approval = sha256(&preview);
    let written: Value =
        serde_json::from_slice(&invoke(&case, "attachment-review", Some(&approval), true)?)?;
    let original: Receipt = serde_json::from_value(written["receipt"].clone())?;
    check!(eq; written["repeated"], false);
    let installed = state(&case.root)?;
    check!(eq; installed.inventory.records, 1);
    check!(eq; &installed.tables, &before.tables);
    check!(eq; &installed.head, &before.head);
    let replay: Value =
        serde_json::from_slice(&invoke(&case, "attachment-replay", Some(&approval), false)?)?;
    let applied: Application = serde_json::from_value(replay["application"].clone())?;
    check!(eq; applied, Application::Repeated(original));
    check!(eq; state(&case.root)?, installed);
    Ok(())
}

#[test]
fn directory_command_paths_refuse_through_normal_review_and_replay_before_store_open() -> TestResult
{
    let mut case = Case::new(false)?;
    let original_root = case.root.clone();
    let before = state(&original_root)?;
    let approval = sha256(&case.command.approval_bytes(4 * 1024 * 1024)?);
    case.root = case.directory.path().join("must-remain-absent");
    case.command_path = case.directory.path().join("command-directory");
    std::fs::create_dir(&case.command_path)?;
    for verb in ["attachment-review", "attachment-replay"] {
        let mut output = Vec::new();
        let error = invoke_to(&case, verb, Some(&approval), false, &mut output)
            .err()
            .ok_or("directory command input accepted")?;
        check!(matches!(error.downcast_ref::<census_crawl::CrawlError>(),
            Some(census_crawl::CrawlError::Io { path, source })
            if path == &case.command_path && source.kind() == std::io::ErrorKind::InvalidInput
                && source.to_string() == "converter input is not a regular file"));
        check!(output.is_empty());
        check!(!case.root.exists());
        check!(eq; state(&original_root)?, before);
    }
    Ok(())
}

#[test]
fn directory_inline_and_retained_leaf_suppliers_refuse_through_normal_cli_before_store_open(
) -> TestResult {
    let mut case = Case::new(false)?;
    let original_root = case.root.clone();
    let before = state(&original_root)?;
    case.root = case.directory.path().join("must-remain-absent");
    let path = case.directory.path().join("supplier-directory");
    std::fs::create_dir(&path)?;
    for retained in [false, true] {
        case.inline.clear();
        case.retained.clear();
        if retained {
            case.retained.push(path.clone());
        } else {
            case.inline.push(path.clone());
        }
        let mut output = Vec::new();
        let error = invoke_to(&case, "attachment-review", None, true, &mut output)
            .err()
            .ok_or("directory supplier accepted")?;
        check!(eq; error.root_cause().to_string(), "attachment material is not a regular file");
        check!(output.is_empty());
        check!(!case.root.exists());
        check!(eq; state(&original_root)?, before);
    }
    Ok(())
}

#[test]
fn malformed_retained_root_key_refuses_locally_before_store_open_without_effects() -> TestResult {
    let mut case = Case::new(true)?;
    let original_root = case.root.clone();
    let before = state(&original_root)?;
    let path = case.retained.first().ok_or("retained original body leaf")?;
    let mut leaf: census_store::RetainedCaptureLeafV1 =
        serde_json::from_slice(&std::fs::read(path)?)?;
    leaf.root.key = "malformed-retained-root-key".into();
    std::fs::write(path, serde_json::to_vec(&leaf)?)?;
    case.root = case.directory.path().join("must-remain-absent");
    let mut output = Vec::new();
    let error = invoke_to(&case, "attachment-review", None, true, &mut output)
        .err()
        .ok_or("malformed retained root key accepted")?;
    check!(eq; error.to_string(), "parsing the retained attachment root key");
    check!(matches!(error.downcast_ref::<census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError>(),
        Some(census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Invalid("SHA256"))));
    check!(output.is_empty());
    check!(!case.root.exists());
    check!(eq; state(&original_root)?, before);
    Ok(())
}

#[test]
fn original_command_with_mismatched_evidence_fence_refuses_without_rewriting_or_effects(
) -> TestResult {
    let case = Case::new(false)?;
    let mut command = case.command.clone();
    command.expected_evidence_generation = command
        .expected_evidence_generation
        .checked_add(1)
        .ok_or("evidence generation overflow")?;
    case.write_command(&command)?;
    let original_bytes = std::fs::read(&case.command_path)?;
    let before = state(&case.root)?;
    let mut output = Vec::new();
    let error = invoke_to(&case, "attachment-review", None, true, &mut output)
        .err()
        .ok_or("mismatched evidence fence accepted")?;
    check!(matches!(
        error.downcast_ref::<census_store::StoreError>(),
        Some(census_store::StoreError::EvidenceSubjectAttachment(
            census_store::EvidenceSubjectAttachmentStoreError::InputMoved
        ))
    ));
    check!(output.is_empty());
    check!(eq; state(&case.root)?, before);
    check!(eq; std::fs::read(&case.command_path)?, original_bytes);
    Ok(())
}
