use super::context::{list, object, text};
use super::support::*;
use census_store::{EvidenceSubjectAttachmentStoreError as AttachmentError, StoreError};
use clap::Parser;
use std::ffi::OsStr;

fn unchanged_refusal(case: &Case, verb: &str, approval: Option<&str>) -> TestResult<anyhow::Error> {
    let before = state(&case.root)?;
    let mut output = Vec::new();
    let error = invoke_to(
        case,
        verb,
        approval,
        verb == "attachment-review",
        &mut output,
    )
    .err()
    .ok_or("invalid attachment input was accepted")?;
    check!(output.is_empty());
    check!(eq; state(&case.root)?, before);
    Ok(error)
}

#[test]
fn replay_only_refuses_a_new_operation_and_a_changed_original_command_without_effects() -> TestResult
{
    let case = Case::new(false)?;
    let preview = invoke(&case, "attachment-review", None, true)?;
    let approval = sha256(&preview);
    let absent = unchanged_refusal(&case, "attachment-replay", Some(&approval))?;
    check!(matches!(
        absent.downcast_ref::<StoreError>(),
        Some(StoreError::EvidenceSubjectAttachment(
            AttachmentError::Corrupt("new attachment requires an inspection")
        ))
    ));
    invoke(&case, "attachment-review", Some(&approval), true)?;
    let mut changed = case.command.clone();
    changed.operation = text("different-never-applied-operation")?;
    case.write_command(&changed)?;
    let new_hash = sha256(&changed.approval_bytes(4 * 1024 * 1024)?);
    let new_operation = unchanged_refusal(&case, "attachment-replay", Some(&new_hash))?;
    check!(matches!(
        new_operation.downcast_ref::<StoreError>(),
        Some(StoreError::EvidenceSubjectAttachment(
            AttachmentError::Corrupt("new attachment requires an inspection")
        ))
    ));
    changed = case.command.clone();
    changed.rationale = text("Changed original command must not match the retained operation")?;
    case.write_command(&changed)?;
    let changed_hash = sha256(&changed.approval_bytes(4 * 1024 * 1024)?);
    let mismatch = unchanged_refusal(&case, "attachment-replay", Some(&changed_hash))?;
    check!(matches!(
        mismatch.downcast_ref::<StoreError>(),
        Some(StoreError::EvidenceSubjectAttachment(
            AttachmentError::OperationConflict
        ))
    ));
    Ok(())
}

#[test]
fn malformed_and_oversized_commands_refuse_before_an_absent_store_is_opened() -> TestResult {
    let mut case = Case::new(false)?;
    let original = state(&case.root)?;
    let existing_root = case.root.clone();
    case.root = case.directory.path().join("must-not-be-created");
    std::fs::write(&case.command_path, b"{not-json")?;
    let mut output = Vec::new();
    let malformed = invoke_to(&case, "attachment-review", None, true, &mut output)
        .err()
        .ok_or("malformed command accepted")?;
    check!(malformed.downcast_ref::<serde_json::Error>().is_some());
    check!(output.is_empty());
    check!(!case.root.exists());
    std::fs::File::create(&case.command_path)?.set_len(4 * 1024 * 1024 + 1)?;
    let oversized = invoke_to(&case, "attachment-review", None, true, &mut output)
        .err()
        .ok_or("oversized command accepted")?;
    check!(format!("{oversized:#}").contains("bytes"));
    check!(output.is_empty());
    check!(!case.root.exists());
    check!(eq; state(&existing_root)?, original);
    Ok(())
}

#[test]
fn invalid_command_version_and_approval_spelling_refuse_without_opening_missing_root() -> TestResult
{
    let mut case = Case::new(false)?;
    case.root = case.directory.path().join("missing-root");
    let mut changed = case.command.clone();
    changed.version = 2;
    case.write_command(&changed)?;
    let mut output = Vec::new();
    let version = invoke_to(&case, "attachment-review", None, true, &mut output)
        .err()
        .ok_or("version two command accepted")?;
    check!(format!("{version:#}").contains("command schema"));
    check!(output.is_empty());
    check!(!case.root.exists());
    case.write_command(&case.command)?;
    for value in [
        "A".repeat(64),
        "0".repeat(63),
        "0".repeat(65),
        format!("{}\n", "0".repeat(64)),
    ] {
        for verb in ["attachment-review", "attachment-replay"] {
            let error = invoke_to(&case, verb, Some(&value), false, &mut output)
                .err()
                .ok_or("invalid SHA spelling accepted")?;
            check!(format!("{error:#}").contains("SHA256"));
            check!(output.is_empty());
            check!(!case.root.exists());
        }
    }
    Ok(())
}

#[test]
fn duplicate_undeclared_changed_and_oversized_materials_leave_the_store_unchanged() -> TestResult {
    let mut case = Case::new(false)?;
    let first = case.inline.first().ok_or("inline fixture missing")?.clone();
    case.inline.push(first.clone());
    let duplicate = unchanged_refusal(&case, "attachment-review", None)?;
    check!(format!("{duplicate:#}").contains("duplicate"));
    check!(case.inline.pop().is_some());
    let undeclared = case.directory.path().join("undeclared.bin");
    std::fs::write(&undeclared, b"undeclared source material")?;
    case.inline.push(undeclared);
    let error = unchanged_refusal(&case, "attachment-review", None)?;
    check!(format!("{error:#}").contains("declared"));
    check!(case.inline.pop().is_some());
    std::fs::write(&first, b"changed declared bytes")?;
    let error = unchanged_refusal(&case, "attachment-review", None)?;
    check!(format!("{error:#}").contains("declared"));
    std::fs::File::create(&first)?.set_len(4 * 1024 * 1024 + 1)?;
    let error = unchanged_refusal(&case, "attachment-review", None)?;
    check!(eq; error.root_cause().to_string(), "attachment material exceeds its file limit");
    Ok(())
}

#[test]
fn incorrect_declared_length_and_too_many_suppliers_refuse_before_opening_store() -> TestResult {
    let mut case = Case::new(false)?;
    case.root = case.directory.path().join("unopened-root");
    let first = std::fs::read(case.inline.first().ok_or("first inline")?)?;
    let reference = object(first)?.reference;
    let mut changed = case.command.clone();
    let mut objects = changed.reviewed_context.objects.as_slice().to_vec();
    let declared = objects
        .iter_mut()
        .find(|held| held.digest == reference.digest)
        .ok_or("declared first reference")?;
    declared.bytes = declared
        .bytes
        .checked_add(1)
        .ok_or("declared length overflow")?;
    changed.reviewed_context.objects = list(objects)?;
    case.write_command(&changed)?;
    let mut output = Vec::new();
    let length = invoke_to(&case, "attachment-review", None, true, &mut output)
        .err()
        .ok_or("declared length mismatch accepted")?;
    check!(format!("{length:#}").contains("length"));
    check!(output.is_empty());
    check!(!case.root.exists());
    case.write_command(&case.command)?;
    let first_path = case.inline.first().ok_or("first inline")?.clone();
    case.inline = vec![first_path; 65];
    let count = invoke_to(&case, "attachment-review", None, true, &mut output)
        .err()
        .ok_or("65 suppliers accepted")?;
    check!(format!("{count:#}").contains("64"));
    check!(output.is_empty());
    check!(!case.root.exists());
    Ok(())
}

#[test]
fn missing_explicit_store_and_replay_approval_refuse_through_normal_cli() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("command.json");
    let cli = crate::cli::Cli::try_parse_from([
        OsStr::new("census-service"),
        OsStr::new("capture-protocol"),
        OsStr::new("attachment-review"),
        OsStr::new("--command"),
        path.as_os_str(),
    ])?;
    let crate::cli::command::Command::CaptureProtocol(args) = &cli.command else {
        return Err("capture-protocol parse mismatch".into());
    };
    let mut output = Vec::new();
    let error = crate::cli::capture_protocol::run_to(cli.store.as_deref(), args, &mut output)
        .err()
        .ok_or("implicit Store root accepted")?;
    check!(eq; error.to_string(), "capture-protocol requires an explicit --store root");
    check!(output.is_empty());
    let error = crate::cli::Cli::try_parse_from([
        OsStr::new("census-service"),
        OsStr::new("capture-protocol"),
        OsStr::new("attachment-replay"),
        OsStr::new("--command"),
        path.as_os_str(),
    ])
    .err()
    .ok_or("replay approval optional")?;
    check!(eq; error.kind(), clap::error::ErrorKind::MissingRequiredArgument);
    check!(!path.exists());
    Ok(())
}
