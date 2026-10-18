use super::*;
use crate::StoreFormat;
use census_domain::model::StoreIdentity;
use fjall::{Keyspace, Readable, Snapshot};

pub(crate) fn validate_format(
    snapshot: &Snapshot,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    format: &StoreFormat,
    budget: &mut ResultSchoolStartupValidationBudgetV1,
) -> StoreResult<()> {
    guarded(&mut budget.grant, |grant| {
        let proof = load(snapshot, meta, journal, receipts, format, grant)?;
        if let Some(proof) = proof {
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
    budget: &mut ResultSchoolStartupValidationBudgetV1,
) -> StoreResult<ResultSchoolPriorCapabilityV1> {
    guarded(&mut budget.grant, |grant| {
        let held = load(snapshot, meta, journal, receipts, format, grant)?
            .ok_or(Error::EnrollmentRequired)?;
        let enrollment: wire::Enrollment = codec::decode(&held.enrollment, grant)?;
        Ok(enrollment.command.prior)
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

pub(super) fn enrollment(
    snapshot: &Snapshot,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<wire::Proof> {
    let enrollment = value(snapshot, meta, ENROLLMENT.as_bytes(), grant)?;
    let record: wire::Enrollment = codec::decode(&enrollment, grant)?;
    let operation = wire::enrollment_operation(&record.command);
    let receipt = value(snapshot, receipts, operation.as_bytes(), grant)?;
    let qualification = qualification(snapshot, meta, grant)?;
    let prior = prior(
        snapshot,
        meta,
        journal,
        receipts,
        &record.command.prior,
        grant,
    )?;
    let proof = wire::Proof {
        schema: "result-school-enrollment-proof-v1".into(),
        enrollment,
        receipt,
        qualification,
        prior,
    };
    super::proof::enrollment::verify(&proof, &identity(snapshot, journal, grant)?, grant)?;
    Ok(proof)
}

pub(super) fn qualification(
    snapshot: &Snapshot,
    meta: &Keyspace,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    value(
        snapshot,
        meta,
        crate::public_identity::DECLARATION.as_bytes(),
        grant,
    )
}

pub(super) fn value(
    snapshot: &Snapshot,
    table: &Keyspace,
    key: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    bounded_value(snapshot, table, key, MAX_RESULT_SCHOOL_RECORD_BYTES, grant)
}

pub(super) fn entry_value(
    snapshot: &Snapshot,
    table: &Keyspace,
    key: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    bounded_value(
        snapshot,
        table,
        key,
        census_domain::public_identity_history::ReplayLimits::SMALL.total_text_bytes,
        grant,
    )
}

fn bounded_value(
    snapshot: &Snapshot,
    table: &Keyspace,
    key: &[u8],
    maximum: usize,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    grant.charge_steps(2).map_err(grant_error)?;
    let length = snapshot
        .size_of(table, key)
        .map_err(|source| StoreError::Read { source })?
        .ok_or(Error::Corrupt("missing protected value"))?;
    let length = usize::try_from(length).map_err(|_| Error::BudgetExceeded)?;
    if length > maximum {
        return Err(Error::BudgetExceeded.into());
    }
    grant
        .charge_bytes(
            length
                .checked_mul(2)
                .and_then(|size| size.checked_add(key.len()))
                .ok_or(Error::BudgetExceeded)?,
        )
        .map_err(grant_error)?;
    let raw = snapshot
        .get(table, key)
        .map_err(|source| StoreError::Read { source })?
        .ok_or(Error::Corrupt("missing protected value"))?;
    if raw.len() != length {
        return Err(Error::Corrupt("protected value size changed").into());
    }
    let mut output = Vec::new();
    output
        .try_reserve_exact(length)
        .map_err(|_| Error::BudgetExceeded)?;
    output.extend_from_slice(&raw);
    Ok(output)
}
pub(super) fn namespace(
    snapshot: &Snapshot,
    table: &Keyspace,
    prefix: &str,
    expected: &[&str],
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let mut rows = snapshot.prefix(table, prefix);
    expected
        .iter()
        .try_for_each(|expected| -> StoreResult<()> {
            grant.charge_steps(1).map_err(grant_error)?;
            let row = rows
                .next()
                .ok_or(Error::Corrupt("missing protected namespace key"))?;
            let key = row.key().map_err(|source| StoreError::Read { source })?;
            grant.charge_bytes(key.len()).map_err(grant_error)?;
            if expected.as_bytes() != key.as_ref() {
                return Err(Error::Corrupt("unexpected protected namespace key").into());
            }
            Ok(())
        })?;
    grant.charge_steps(1).map_err(grant_error)?;
    if let Some(row) = rows.next() {
        row.key().map_err(|source| StoreError::Read { source })?;
        return Err(Error::Corrupt("extra protected namespace key").into());
    }
    Ok(())
}

pub(super) fn prior(
    snapshot: &Snapshot,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    capability: &ResultSchoolPriorCapabilityV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<wire::PriorProof> {
    match capability {
        ResultSchoolPriorCapabilityV1::IdentityOnly { .. } => {
            [meta, journal, receipts]
                .into_iter()
                .try_for_each(|table| {
                    namespace(snapshot, table, "roster_source_issuance", &[], grant)
                })?;
            Ok(wire::PriorProof::IdentityOnly)
        }
        ResultSchoolPriorCapabilityV1::IdentityAndRosterSource { .. } => {
            let enrollment = value(
                snapshot,
                meta,
                crate::roster_source_issuance::ENROLLMENT.as_bytes(),
                grant,
            )?;
            let operation = crate::roster_source_issuance::retained_enrollment_operation_v1(
                &enrollment,
                grant,
            )?;
            let receipt = value(snapshot, receipts, operation.as_bytes(), grant)?;
            Ok(wire::PriorProof::IdentityAndRosterSource {
                enrollment,
                receipt,
            })
        }
    }
}

pub(super) fn identity(
    snapshot: &Snapshot,
    journal: &Keyspace,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<StoreIdentity> {
    let key = crate::Store::journal_key("export-lineage-v1", "store-identity");
    value(snapshot, journal, &key, grant)?;
    grant.charge_steps(64).map_err(grant_error)?;
    match crate::lineage::read(snapshot, journal)? {
        crate::lineage::RetainedIdentity::Present(identity) => Ok(identity),
        crate::lineage::RetainedIdentity::Absent => {
            Err(Error::Corrupt("origin lineage absent").into())
        }
    }
}

pub(super) fn snapshot_format(
    snapshot: &Snapshot,
    meta: &Keyspace,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<StoreFormat> {
    let mut lifecycle_revision = None;
    [
        ("schema:store_version", Some(b"2".as_slice())),
        ("schema:key_format", Some(b"1".as_slice())),
        ("schema:lifecycle_revision", None),
        ("schema:migrating_to", None),
    ]
    .into_iter()
    .try_for_each(|(key, expected)| -> StoreResult<()> {
        grant.charge_steps(2).map_err(grant_error)?;
        let size = snapshot
            .size_of(meta, key)
            .map_err(|source| StoreError::Read { source })?;
        if size.is_some_and(|size| size > 32) {
            return Err(Error::BudgetExceeded.into());
        }
        grant.charge_bytes(32).map_err(grant_error)?;
        let raw = snapshot
            .get(meta, key)
            .map_err(|source| StoreError::Read { source })?;
        let bytes = raw.as_ref().map(|raw| raw.as_ref());
        if key == "schema:lifecycle_revision" {
            lifecycle_revision = match bytes {
                Some(b"7") => Some(7),
                Some(b"8") => Some(8),
                Some(b"9") => Some(9),
                _ => return Err(Error::EnrollmentRequired.into()),
            };
        } else if bytes != expected {
            return Err(Error::EnrollmentRequired.into());
        }
        Ok(())
    })?;
    Ok(StoreFormat {
        schema_version: Some(2),
        key_format: Some(1),
        lifecycle_revision,
        created_by: None,
        created_at: None,
        migration_target: None,
    })
}
