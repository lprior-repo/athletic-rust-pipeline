use super::*;
use std::io::Write;

fn corrupt_same_length_and_mtime(path: &std::path::Path) -> TestResult<Vec<u8>> {
    let before = std::fs::metadata(path)?;
    let mut bytes = std::fs::read(path)?;
    let first = bytes.first_mut().ok_or("nonempty fixture")?;
    *first ^= 1;
    let mut file = std::fs::OpenOptions::new().write(true).open(path)?;
    file.write_all(&bytes)?;
    file.set_times(std::fs::FileTimes::new().set_modified(before.modified()?))?;
    file.sync_all()?;
    let after = std::fs::metadata(path)?;
    check!(eq; after.len(), before.len());
    check!(eq; after.modified()?, before.modified()?);
    Ok(bytes)
}

#[test]
fn prepared_session_callback_cannot_reuse_stale_exact_body_or_metadata() -> TestResult {
    for occupied in ["body", "metadata"] {
        let (dir, store, writer, owner) = fixture_for(CaptureProtocolVersion::V2)?;
        let (prepared, body) = prepared(&store, &writer, &owner, 200)?;
        let (body_path, metadata_path) = publish(&store, &prepared, &body)?;
        let candidate = dir.path().join("candidate");
        std::fs::write(&candidate, &body)?;
        let (target, usage) = selected(&store, &owner)?;
        if occupied == "body" {
            std::fs::remove_file(&metadata_path)?;
        } else {
            std::fs::remove_file(&body_path)?;
        }
        let before = rows(&store)?;
        let mut parent = PreparedRecoveryBudget::admit(usage, 1)?;
        let mut session = unresolved(
            store
                .begin_prepared_recovery(&owner, &target, &mut parent)?
                .read_backing()?,
        )?;
        session.prepare_restore(ExactBodySource::SuppliedLocalCopy(&candidate))?;
        let changed_path = if occupied == "body" {
            &body_path
        } else {
            &metadata_path
        };
        let changed;
        {
            let (_fence, capture, held_body, mut io) = session.restoration_io()?;
            io.direct_operation(RecoveryFsOperation::Open)?;
            changed = corrupt_same_length_and_mtime(changed_path)?;
            io.direct_operation(RecoveryFsOperation::Create)?;
            if occupied == "body" {
                std::fs::write(&metadata_path, capture.metadata_bytes())?;
            } else {
                std::fs::write(&body_path, held_body)?;
            }
        }
        check!(matches!(
            session.verify_restored(),
            Err(StoreError::Capture(CaptureError::PreparedRepairRequired))
        ));
        check!(matches!(
            session.finish(),
            Err(StoreError::Capture(CaptureError::TransitionRefused))
        ));
        check!(eq; std::fs::read(changed_path)?, changed);
        check!(eq; rows(&store)?, before);
    }
    Ok(())
}

#[test]
fn prepared_restoration_fresh_checks_fit_six_eof_probes_and_refuse_five_before_commit() -> TestResult
{
    for allowance in [6, 5] {
        let (dir, store, writer, owner) = fixture_for(CaptureProtocolVersion::V2)?;
        let (prepared, body) = prepared(&store, &writer, &owner, 200)?;
        let (body_path, _) = publish(&store, &prepared, &body)?;
        std::fs::remove_file(&body_path)?;
        let candidate = dir.path().join("candidate");
        std::fs::write(&candidate, &body)?;
        let (target, usage) = selected(&store, &owner)?;
        let mut parent = PreparedRecoveryBudget::admit(usage, 1)?;
        parent.limit_recovery_eof(allowance);
        let mut session = unresolved(
            store
                .begin_prepared_recovery(&owner, &target, &mut parent)?
                .read_backing()?,
        )?;
        session.prepare_restore(ExactBodySource::SuppliedLocalCopy(&candidate))?;
        {
            let (_fence, _capture, held_body, mut io) = session.restoration_io()?;
            io.direct_operation(RecoveryFsOperation::Create)?;
            std::fs::write(&body_path, held_body)?;
        }
        session.verify_restored()?;
        let before = rows(&store)?;
        let result = session.finish();
        if allowance == 6 {
            check!(eq; result?.into_parts().body, body);
        } else {
            check!(matches!(
                result,
                Err(StoreError::Capture(CaptureError::ReadBudgetExceeded {
                    resource: "recovery EOF probes"
                }))
            ));
            check!(eq; rows(&store)?, before);
        }
    }
    Ok(())
}

#[test]
fn prepared_session_finish_freshly_hashes_both_leaves_after_store_verification() -> TestResult {
    for changed_leaf in ["body", "metadata"] {
        let (_dir, store, writer, owner) = fixture_for(CaptureProtocolVersion::V2)?;
        let (prepared, body) = prepared(&store, &writer, &owner, 200)?;
        let (body_path, metadata_path) = publish(&store, &prepared, &body)?;
        let (target, usage) = selected(&store, &owner)?;
        let mut parent = PreparedRecoveryBudget::admit(usage, 1)?;
        let session = unresolved(
            store
                .begin_prepared_recovery(&owner, &target, &mut parent)?
                .read_backing()?,
        )?;
        let changed_path = if changed_leaf == "body" {
            &body_path
        } else {
            &metadata_path
        };
        let changed = corrupt_same_length_and_mtime(changed_path)?;
        let before = rows(&store)?;
        check!(matches!(
            session.finish(),
            Err(StoreError::Capture(CaptureError::PreparedRepairRequired))
        ));
        check!(eq; std::fs::read(changed_path)?, changed);
        check!(eq; rows(&store)?, before);
    }
    Ok(())
}

#[test]
fn missing_close_checks_late_occupied_metadata_before_exact_body_classification() -> TestResult {
    for initially_exact_metadata in [false, true] {
        let (_dir, store, writer, owner) = fixture_for(CaptureProtocolVersion::V2)?;
        let (prepared, body) = prepared(&store, &writer, &owner, 200)?;
        let (body_path, metadata_path) = publish(&store, &prepared, &body)?;
        std::fs::remove_file(&body_path)?;
        if !initially_exact_metadata {
            std::fs::remove_file(&metadata_path)?;
        }
        let (target, usage) = selected(&store, &owner)?;
        let mut parent = PreparedRecoveryBudget::admit(usage, 1)?;
        let mut session = unresolved(
            store
                .begin_prepared_recovery(&owner, &target, &mut parent)?
                .read_backing()?,
        )?;
        let decision = session.missing_decision()?;
        std::fs::write(&body_path, &body)?;
        if !initially_exact_metadata {
            std::fs::write(&metadata_path, prepared.prepared().metadata_bytes())?;
        }
        let changed = corrupt_same_length_and_mtime(&metadata_path)?;
        let before = rows(&store)?;
        check!(matches!(
            session.close_missing(&decision),
            Err(StoreError::Capture(CaptureError::PreparedRepairRequired))
        ));
        check!(eq; std::fs::read(&metadata_path)?, changed);
        check!(eq; std::fs::read(&body_path)?, body);
        check!(eq; rows(&store)?, before);
    }
    Ok(())
}

#[test]
fn recorded_loss_replay_refuses_unsafe_or_missing_root_without_changing_truth() -> TestResult {
    for unsafe_root in [false, true] {
        let (_dir, store, writer, owner) = fixture_for(CaptureProtocolVersion::V2)?;
        prepared(&store, &writer, &owner, 200)?;
        let (target, usage) = selected(&store, &owner)?;
        let mut parent = PreparedRecoveryBudget::admit(usage, 1)?;
        let mut session = unresolved(
            store
                .begin_prepared_recovery(&owner, &target, &mut parent)?
                .read_backing()?,
        )?;
        let decision = session.missing_decision()?;
        session.close_missing(&decision)?;
        let http = store.http_cache_dir();
        let saved = store.root().join("saved-http");
        std::fs::rename(&http, &saved)?;
        if unsafe_root {
            std::fs::write(&http, b"occupied")?;
        }
        let before = rows(&store)?;
        let mut replay = PreparedRecoveryBudget::admit(usage, 1)?;
        let result = store.begin_prepared_recovery(&owner, &target, &mut replay);
        if unsafe_root {
            check!(matches!(
                result,
                Err(StoreError::Capture(CaptureError::PreparedRepairRequired))
            ));
        } else {
            check!(matches!(
                result,
                Err(StoreError::Capture(CaptureError::RequiredBackingMissing))
            ));
        }
        check!(eq; rows(&store)?, before);
    }
    Ok(())
}
