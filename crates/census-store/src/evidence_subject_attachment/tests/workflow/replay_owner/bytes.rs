use super::*;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::public_identity_history::ReplayLimits;
use serde::Deserialize;
use serde_json::value::RawValue;

const AGGREGATE_MAXIMUM: usize = AttachmentReplayGrantV1::MAX_ENCODED_BYTES;
const APPROVAL_MAXIMUM: usize = ReplayLimits::SMALL.total_text_bytes;

#[derive(Deserialize)]
struct Envelope {
    payload: Box<RawValue>,
}

struct ByteWork {
    refresh: usize,
    load: usize,
    approval: usize,
}

fn sum(values: &[usize]) -> TestResult<usize> {
    values.iter().try_fold(0usize, |total, value| {
        total
            .checked_add(*value)
            .ok_or_else(|| "replay byte sum overflow".into())
    })
}

fn work(case: &Committed) -> TestResult<ByteWork> {
    let raw = case
        .fixture
        .store
        .journal
        .get(Store::journal_key("evidence_subject", RECORD_KEY))?
        .ok_or("replay inline root absent")?;
    check!(raw.len() <= crate::MAX_JOURNAL_VALUE_BYTES);
    let entry: Envelope = serde_json::from_slice(&raw)?;
    let payload: serde_json::Value = serde_json::from_str(entry.payload.get())?;
    check!(eq; payload.get("record"), Some(&serde_json::to_value(&case.record)?));
    let receipt = case
        .fixture
        .store
        .receipts
        .get(case.receipt.operation.as_bytes())?
        .ok_or("replay receipt absent")?;
    let record_bytes = serde_json::to_vec(&case.record)?.len();
    let binding = AttachmentReceiptBindingV1 {
        operation: case.record.command.operation.clone(),
        record_digest: case.record.record_digest.clone(),
    };
    let refresh = sum(&[
        RECORD_KEY.len(),
        receipt.len(),
        record_bytes,
        serde_json::to_vec(&binding)?.len(),
    ])?;
    Ok(ByteWork {
        refresh,
        load: sum(&[refresh, entry.payload.get().len(), record_bytes])?,
        approval: case.record.command.approval_bytes(APPROVAL_MAXIMUM)?.len(),
    })
}

fn leave_bytes(grant: &mut AttachmentReplayGrantV1, remaining: usize) -> TestResult {
    let padding = AGGREGATE_MAXIMUM
        .checked_sub(grant.usage().encoded_bytes())
        .and_then(|value| value.checked_sub(remaining))
        .ok_or("replay byte padding underflow")?;
    grant.charge_bytes(padding)?;
    Ok(())
}

fn capacity(
    result: crate::StoreResult<crate::PreparedEvidenceSubjectAttachmentV1<'_>>,
) -> TestResult {
    check!(matches!(
        result,
        Err(StoreError::EvidenceSubjectAttachment(
            AttachmentError::Domain(EvidenceSubjectAttachmentError::Capacity {
                resource: "aggregate encoded bytes",
                needed,
                available
            })
        )) if available == AGGREGATE_MAXIMUM && needed.checked_sub(available) == Some(1)
    ));
    Ok(())
}

#[test]
fn replay_preparation_debits_loaded_owner_caller_owner_approval_and_digest_separately() -> TestResult
{
    let case = committed("replay-owner-byte-ledger")?;
    let work = work(&case)?;
    let before = retained_state(&case.fixture.store)?;
    let prepared = case.fixture.store.prepare_evidence_subject_attachment_v1(
        case.record.command.clone(),
        Preparation::ReplayOnly,
    )?;
    check!(eq; prepared.grant.usage().encoded_bytes(), sum(&[work.load, work.approval, work.approval, 64])?);
    check!(eq; prepared.command(), &case.record.command);
    check!(eq; apply(&case.fixture.store, prepared)?, Application::Repeated(case.receipt));
    check!(eq; retained_state(&case.fixture.store)?, before);
    Ok(())
}

#[test]
fn replay_caller_owner_byte_admission_refuses_before_the_digest_copy() -> TestResult {
    let case = committed("replay-owner-caller-byte-boundary")?;
    let work = work(&case)?;
    let mut inspection = case.fixture.inspect()?;
    let remaining = sum(&[
        work.refresh,
        work.approval,
        work.approval.checked_sub(1).ok_or("approval absent")?,
    ])?;
    leave_bytes(
        inspection.grant.as_mut().ok_or("inspection grant absent")?,
        remaining,
    )?;
    let before = retained_state(&case.fixture.store)?;
    capacity(case.fixture.store.prepare_evidence_subject_attachment_v1(
        case.record.command.clone(),
        Preparation::New {
            inspection,
            context: &[],
        },
    ))?;
    check!(eq; retained_state(&case.fixture.store)?, before);
    Ok(())
}

#[test]
fn replay_digest_byte_admission_refuses_after_the_caller_owner_debit() -> TestResult {
    let case = committed("replay-owner-digest-byte-boundary")?;
    let work = work(&case)?;
    let mut inspection = case.fixture.inspect()?;
    leave_bytes(
        inspection.grant.as_mut().ok_or("inspection grant absent")?,
        sum(&[work.refresh, work.approval, work.approval, 63])?,
    )?;
    let before = retained_state(&case.fixture.store)?;
    capacity(case.fixture.store.prepare_evidence_subject_attachment_v1(
        case.record.command.clone(),
        Preparation::New {
            inspection,
            context: &[],
        },
    ))?;
    check!(eq; retained_state(&case.fixture.store)?, before);
    Ok(())
}

#[test]
fn exact_replay_preparation_byte_limit_keeps_sticky_original_grant_poison() -> TestResult {
    let case = committed("replay-owner-exact-preparation-bytes")?;
    let work = work(&case)?;
    let mut inspection = case.fixture.inspect()?;
    leave_bytes(
        inspection.grant.as_mut().ok_or("inspection grant absent")?,
        sum(&[work.refresh, work.approval, work.approval, 64])?,
    )?;
    let before = retained_state(&case.fixture.store)?;
    let mut prepared = case.fixture.store.prepare_evidence_subject_attachment_v1(
        case.record.command.clone(),
        Preparation::New {
            inspection,
            context: &[],
        },
    )?;
    check!(prepared.is_replay());
    check!(eq; prepared.grant.usage().encoded_bytes(), AGGREGATE_MAXIMUM);
    check!(matches!(
        prepared.grant.charge_bytes(1),
        Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "aggregate encoded bytes",
            needed,
            available
        }) if available == AGGREGATE_MAXIMUM && needed.checked_sub(available) == Some(1)
    ));
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    check!(matches!(
        case.fixture
            .store
            .apply_evidence_subject_attachment_v1(prepared, &approval, &digest),
        Err(StoreError::EvidenceSubjectAttachment(
            AttachmentError::Domain(EvidenceSubjectAttachmentError::Poisoned)
        ))
    ));
    check!(eq; retained_state(&case.fixture.store)?, before);
    Ok(())
}

fn apply_boundary(one_short: bool) -> TestResult {
    let case = committed("replay-owner-application-byte-boundary")?;
    let work = work(&case)?;
    let mut prepared = case.fixture.store.prepare_evidence_subject_attachment_v1(
        case.record.command.clone(),
        Preparation::ReplayOnly,
    )?;
    let remaining = if one_short {
        work.refresh.checked_sub(1).ok_or("refresh bytes absent")?
    } else {
        work.refresh
    };
    leave_bytes(&mut prepared.grant, remaining)?;
    let before = retained_state(&case.fixture.store)?;
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    let result = case
        .fixture
        .store
        .apply_evidence_subject_attachment_v1(prepared, &approval, &digest);
    if one_short {
        check!(matches!(
            result,
            Err(StoreError::EvidenceSubjectAttachment(
                AttachmentError::Domain(EvidenceSubjectAttachmentError::Capacity {
                    resource: "aggregate encoded bytes",
                    needed,
                    available
                })
            )) if available == AGGREGATE_MAXIMUM && needed.checked_sub(available) == Some(1)
        ));
    } else {
        check!(eq; result?, Application::Repeated(case.receipt));
    }
    check!(eq; retained_state(&case.fixture.store)?, before);
    Ok(())
}

#[test]
fn unchanged_replay_apply_completes_with_only_the_exact_fresh_refresh_byte_allowance() -> TestResult
{
    apply_boundary(false)
}

#[test]
fn unchanged_replay_apply_one_byte_short_refuses_with_complete_preservation() -> TestResult {
    apply_boundary(true)
}
