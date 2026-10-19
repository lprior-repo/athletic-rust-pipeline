use super::*;
use crate::StoreFormat;
use census_domain::result_school_correspondence::ResultSchoolPriorCapabilityV1;
use fjall::{Keyspace, Snapshot};

mod physical;
pub(super) use physical::{entry_value, identity, namespace, snapshot_format, value};

pub(crate) fn validate_format(
    snapshot: &Snapshot,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    format: &StoreFormat,
    budget: &mut ResultCalendarStartupValidationBudgetV1,
) -> StoreResult<()> {
    guarded(&mut budget.grant, |grant| {
        if let Some(proof) = load(snapshot, meta, journal, receipts, format, grant)? {
            super::issuance::ledger::load(snapshot, meta, journal, receipts, proof, grant)?;
        }
        Ok(())
    })
}

pub(crate) fn preserved_prior_capability(
    snapshot: &Snapshot,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    format: &StoreFormat,
    budget: &mut ResultCalendarStartupValidationBudgetV1,
) -> StoreResult<ResultSchoolPriorCapabilityV1> {
    guarded(&mut budget.grant, |grant| {
        let proof = load(snapshot, meta, journal, receipts, format, grant)?
            .ok_or(Error::EnrollmentRequired)?;
        super::proof::enrollment::verify(&proof, &identity(snapshot, journal, grant)?, grant)
    })
}

pub(super) fn load(
    snapshot: &Snapshot,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    format: &StoreFormat,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Option<wire::Proof>> {
    if format.lifecycle_revision != Some(LIFECYCLE) {
        [meta, journal, receipts]
            .into_iter()
            .try_for_each(|table| namespace(snapshot, table, NAME, &[], grant))?;
        return Ok(None);
    }
    if format.schema_version != Some(2)
        || format.key_format != Some(1)
        || format.migration_target.is_some()
    {
        return Err(Error::EnrollmentRequired.into());
    }
    namespace(snapshot, meta, NAME, &[ENROLLMENT, wire::HEAD], grant)?;
    enrollment(snapshot, meta, journal, receipts, grant).map(Some)
}

pub(super) fn school_proof(
    snapshot: &Snapshot,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    crate::result_school_correspondence::retained_enrollment_bytes_v1(
        snapshot, meta, journal, receipts, grant,
    )
}

pub(super) fn enrollment(
    snapshot: &Snapshot,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<wire::Proof> {
    let bytes = value(snapshot, meta, ENROLLMENT.as_bytes(), grant)?;
    let record: wire::Enrollment = codec::decode(&bytes, grant)?;
    let receipt = value(
        snapshot,
        receipts,
        wire::enrollment_operation(&record.command).as_bytes(),
        grant,
    )?;
    let current_school = school_proof(snapshot, meta, journal, receipts, grant)?;
    if codec::digest(&current_school, grant)? != record.command.prior_result_school_enrollment {
        return Err(Error::Corrupt("retained school predecessor moved").into());
    }
    let proof = wire::Proof {
        schema: "result-calendar-enrollment-proof-v1".into(),
        enrollment: codec::raw(bytes, grant)?,
        receipt: codec::raw(receipt, grant)?,
    };
    let store = identity(snapshot, journal, grant)?;
    super::proof::enrollment::verify(&proof, &store, grant)?;
    Ok(proof)
}
