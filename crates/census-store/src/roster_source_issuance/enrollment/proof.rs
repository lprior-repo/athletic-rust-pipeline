use super::super::{
    codec, source_error, RosterSourceIssuanceStoreError as Error, OPERATION_PREFIX,
};
use super::{approval, EnrollmentRecord};
use crate::{Receipt, StoreResult};
use census_domain::{evidence_subject_attachment::AttachmentReplayGrantV1, model::StoreIdentity};

pub(crate) fn retained_enrollment_operation_v1(
    bytes: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<String> {
    with_grant(grant, |grant| {
        admit(bytes, &[], grant)?;
        let enrolled: EnrollmentRecord = codec::decode(bytes)?;
        super::prepare::validate_command(&enrolled.command)?;
        if enrolled.schema != "roster-source-enrollment-v1" {
            return Err(Error::Corrupt("enrollment authority mismatch").into());
        }
        Ok(format!(
            "{OPERATION_PREFIX}{}",
            enrolled.command.operation.as_str()
        ))
    })
}

pub(crate) fn validate_roster_enrollment_proof_v1(
    enrollment: &[u8],
    receipt: &[u8],
    identity: &StoreIdentity,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    with_grant(grant, |grant| {
        admit(enrollment, receipt, grant)?;
        let enrolled: EnrollmentRecord = codec::decode(enrollment)?;
        validate(&enrolled, enrollment, receipt, identity).map(|_| ())
    })
}

pub(in super::super) fn validate(
    enrolled: &EnrollmentRecord,
    enrollment: &[u8],
    receipt: &[u8],
    identity: &StoreIdentity,
) -> StoreResult<String> {
    let receipt: Receipt = codec::decode(receipt)?;
    let operation = format!("{OPERATION_PREFIX}{}", enrolled.command.operation.as_str());
    let enrollment_digest = codec::digest(enrollment);
    let approval = approval(&enrolled.command)?;
    super::prepare::validate_command(&enrolled.command)?;
    if &enrolled.command.store != identity
        || enrolled.schema != "roster-source-enrollment-v1"
        || enrolled.approval_digest != codec::digest(&approval)
        || enrolled.committed_at < enrolled.command.reviewed_at
        || receipt.operation != operation
        || receipt.digest != enrollment_digest
        || receipt.at != enrolled.committed_at.to_rfc3339()
        || receipt.appended != 0
        || receipt.credited
    {
        return Err(Error::Corrupt("enrollment authority mismatch").into());
    }
    Ok(enrollment_digest)
}

fn admit(left: &[u8], right: &[u8], grant: &mut AttachmentReplayGrantV1) -> StoreResult<()> {
    if left.len() > codec::MAX_BYTES || right.len() > codec::MAX_BYTES {
        return Err(Error::BudgetExceeded.into());
    }
    let size = left
        .len()
        .checked_add(right.len())
        .ok_or(Error::BudgetExceeded)?;
    let bytes = size.checked_mul(6).ok_or(Error::BudgetExceeded)?;
    let steps = size
        .div_ceil(32)
        .checked_add(128)
        .ok_or(Error::BudgetExceeded)?;
    grant.charge_steps(steps).map_err(source_error)?;
    grant.charge_bytes(bytes).map_err(source_error)
}

fn with_grant<T>(
    grant: &mut AttachmentReplayGrantV1,
    f: impl FnOnce(&mut AttachmentReplayGrantV1) -> StoreResult<T>,
) -> StoreResult<T> {
    grant.ensure_available().map_err(source_error)?;
    let result = f(grant);
    if result.is_err() {
        grant.poison();
    }
    result
}
