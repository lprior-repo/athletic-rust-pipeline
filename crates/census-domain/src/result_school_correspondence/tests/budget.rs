use super::*;
use crate::public_identity_history::ReplayLimits;

#[test]
fn measured_digest_exact_byte_and_step_caps_preserve_original_budget() -> TestResult {
    let value = command("controlled-budget")?;
    let mut probe = AttachmentReplayGrantV1::new();
    let expected = result_school_digest_v1(&value, &mut probe)?;
    let usage = probe.usage();
    for bytes in [true, false] {
        for short in [0usize, 1] {
            let mut grant = AttachmentReplayGrantV1::new();
            if bytes {
                grant.charge_bytes(
                    ReplayLimits::SMALL
                        .total_text_bytes
                        .checked_sub(usage.encoded_bytes())
                        .and_then(|n| n.checked_add(short))
                        .ok_or("byte cap")?,
                )?;
            } else {
                grant.charge_steps(
                    ReplayLimits::SMALL
                        .traversal_steps
                        .checked_sub(usage.traversal_steps())
                        .and_then(|n| n.checked_add(short))
                        .ok_or("step cap")?,
                )?;
            }
            let result = result_school_digest_v1(&value, &mut grant);
            if short == 0 {
                check!(eq; result?, expected);
                check!(grant.ensure_available().is_ok());
            } else {
                check!(result.is_err());
                check!(grant.ensure_available().is_err());
            }
        }
    }
    Ok(())
}

#[test]
fn canonical_record_byte_limit_is_exact_and_oversize_poisoned() -> TestResult {
    let exact = "x".repeat(
        MAX_RESULT_SCHOOL_RECORD_BYTES
            .checked_sub(2)
            .ok_or("json quotes")?,
    );
    let mut grant = AttachmentReplayGrantV1::new();
    check!(result_school_digest_v1(&exact, &mut grant).is_ok());
    let mut grant = AttachmentReplayGrantV1::new();
    check!(result_school_digest_v1(&format!("{exact}x"), &mut grant).is_err());
    check!(grant.ensure_available().is_err());
    Ok(())
}

#[test]
fn exhausted_or_foreign_grant_cannot_prepare_or_reuse_checked_command() -> TestResult {
    let mut original = AttachmentReplayGrantV1::new();
    let checked =
        CheckedResultSchoolCommandV1::check(command("controlled-bound-grant")?, &mut original)?;
    let mut foreign = AttachmentReplayGrantV1::new();
    check!(prepare_result_school_record_v1(checked, stamp()?, &mut foreign).is_err());
    check!(foreign.ensure_available().is_err());
    check!(original.ensure_available().is_ok());
    let checked =
        CheckedResultSchoolCommandV1::check(command("controlled-exhausted")?, &mut original)?;
    original.charge_steps(
        ReplayLimits::SMALL
            .traversal_steps
            .checked_sub(original.usage().traversal_steps())
            .ok_or("remaining steps")?,
    )?;
    check!(prepare_result_school_record_v1(checked, stamp()?, &mut original).is_err());
    check!(original.ensure_available().is_err());
    Ok(())
}
