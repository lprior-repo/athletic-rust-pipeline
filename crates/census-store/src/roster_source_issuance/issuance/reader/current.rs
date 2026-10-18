use super::super::super::{codec, format, RosterStartupValidationBudgetV1};
use super::*;
use crate::read::JournalNamespace;
use crate::{PhysicalReadAdmissionV1, ReadBudget, StoreError};
use fjall::Readable;
use std::collections::BTreeMap;

pub(super) fn read(
    view: &StoreSnapshot<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<CurrentRosterReviewedClaimsV1> {
    let continuation = grant
        .checkpoint()
        .map_err(super::super::super::source_error)?;
    let mut archive = capture_archive(view, grant)?;
    let roots = roots(&archive, grant)?;
    for root in roots {
        continuation
            .require(grant)
            .map_err(super::super::super::source_error)?;
        archive.external.push(capture(view, root, grant)?);
    }
    let claims = reconstruct::claims(&archive, grant)?;
    continuation
        .require(grant)
        .map_err(super::super::super::source_error)?;
    Ok(CurrentRosterReviewedClaimsV1 {
        archive,
        claims,
        continuation,
    })
}

fn capture_archive(
    view: &StoreSnapshot<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Archive> {
    let (lifecycle, metadata, records, receipts) = snapshot_protected(view, grant)?;
    grant
        .charge_bytes(1024)
        .map_err(super::super::super::source_error)?;
    grant
        .charge_steps(8)
        .map_err(super::super::super::source_error)?;
    let store = view.store_identity()?;
    let physical = physical_snapshot(view, grant)?;
    grant
        .charge_bytes(1024)
        .map_err(super::super::super::source_error)?;
    grant
        .charge_steps(8)
        .map_err(super::super::super::source_error)?;
    let current_input_token = view.attachment_input_revision_bytes()?;
    let inherited_enrollment_proof = inherited_proof(view, lifecycle, grant)?;
    let archive = Archive {
        schema: if inherited_enrollment_proof.is_some() {
            super::INHERITED_ARCHIVE_SCHEMA.into()
        } else {
            super::ARCHIVE_SCHEMA.into()
        },
        store,
        selected_snapshot_sequence: view.sequence(),
        source_store_lifecycle: lifecycle,
        current_input_token,
        metadata,
        records,
        receipts,
        physical,
        external: Vec::new(),
        inherited_enrollment_proof,
    };
    Ok(archive)
}

fn snapshot_protected(
    view: &StoreSnapshot<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<(u32, format::Rows, format::Rows, format::Rows)> {
    view.with_roster_source_snapshot_v1(|snapshot, meta, journal, receipts| {
        let lifecycle = lifecycle(snapshot, meta, grant)?;
        let mut budget = RosterStartupValidationBudgetV1::new();
        let metadata = format::protected_rows(snapshot, meta, &mut budget, Some(&mut *grant))?;
        let records = format::protected_rows(snapshot, journal, &mut budget, Some(&mut *grant))?;
        let receipts = format::scan(
            snapshot,
            receipts,
            super::super::super::OPERATION_PREFIX,
            &mut budget,
            Some(&mut *grant),
        )?;
        Ok((lifecycle, metadata, records, receipts))
    })
}

fn physical_snapshot(
    view: &StoreSnapshot<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<PhysicalRow>> {
    let rows = super::super::super::inspection::inventory::collect(view, grant)?;
    Ok(rows
        .into_iter()
        .map(|row| {
            let (table, key, value) = row.into_parts();
            PhysicalRow { table, key, value }
        })
        .collect())
}

fn lifecycle(
    snapshot: &fjall::Snapshot,
    meta: &fjall::Keyspace,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<u32> {
    let mut lifecycle = None;
    for (key, expected) in [
        ("schema:store_version", Some(b"2".as_slice())),
        ("schema:key_format", Some(b"1".as_slice())),
        ("schema:lifecycle_revision", None),
        ("schema:migrating_to", None),
    ] {
        grant
            .charge_steps(2)
            .map_err(super::super::super::source_error)?;
        grant
            .charge_bytes(32)
            .map_err(super::super::super::source_error)?;
        let size = snapshot
            .size_of(meta, key)
            .map_err(|source| StoreError::Read { source })?;
        if size.is_some_and(|size| size > 32) {
            return Err(Error::BudgetExceeded.into());
        }
        let value = snapshot
            .get(meta, key)
            .map_err(|source| StoreError::Read { source })?;
        if key == "schema:lifecycle_revision" {
            lifecycle = match value.as_ref().map(|raw| raw.as_ref()) {
                Some(b"8") => Some(super::super::super::LIFECYCLE),
                Some(b"9") => Some(crate::RESULT_SCHOOL_CORRESPONDENCE_LIFECYCLE),
                _ => return Err(Error::EnrollmentRequired.into()),
            };
        } else if value.as_ref().map(|raw| raw.as_ref()) != expected {
            return Err(Error::EnrollmentRequired.into());
        }
    }
    lifecycle.ok_or_else(|| Error::EnrollmentRequired.into())
}

fn inherited_proof(
    view: &StoreSnapshot<'_>,
    lifecycle: u32,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Option<Vec<u8>>> {
    if lifecycle == super::super::super::LIFECYCLE {
        return Ok(None);
    }
    let proof = view
        .result_school_enrollment_proof_v1(grant)?
        .ok_or(Error::EnrollmentRequired)?;
    grant
        .charge_bytes(proof.bytes().len())
        .map_err(super::super::super::source_error)?;
    Ok(Some(proof.bytes().to_vec()))
}

fn roots(
    archive: &Archive,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<crate::RetainedCaptureRootV1>> {
    let objects = reconstruct::objects(archive, grant)?;
    let mut roots = BTreeMap::new();
    for (key, raw) in &archive.records {
        if !key.starts_with(super::super::RECORD_PREFIX.as_bytes()) {
            continue;
        }
        grant
            .charge_bytes(raw.len())
            .map_err(super::super::super::source_error)?;
        grant
            .charge_steps(1)
            .map_err(super::super::super::source_error)?;
        let record: ReviewRecord = codec::decode(raw)?;
        let proposal = super::super::proposal(&record.basis.proposal)?;
        let admission = reconstruct::admission(&proposal, &objects, grant)?;
        let key = (admission.root.schema.phase(), admission.root.key.clone());
        if let Some(prior) = roots.insert(key, admission.root.clone()) {
            if prior != admission.root {
                return Err(Error::Corrupt("review root disagreement").into());
            }
        }
    }
    Ok(roots.into_values().collect())
}

fn capture(
    view: &StoreSnapshot<'_>,
    root: crate::RetainedCaptureRootV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<frozen_source::CapturedExternalPrefix> {
    let remaining = AttachmentReplayGrantV1::MAX_ENCODED_BYTES
        .checked_sub(grant.usage().encoded_bytes())
        .ok_or(Error::BudgetExceeded)?;
    let mut context = (grant, Vec::new());
    view.scan_journal_namespace_preadmitted(
        JournalNamespace::new(root.schema.phase(), &root.key)?,
        ReadBudget {
            rows: 6,
            bytes: u64::try_from(remaining)
                .map_err(|_| Error::BudgetExceeded)?
                .min(super::MAX_EXTERNAL_CAPTURE_BYTES),
            record_bytes: crate::MAX_JOURNAL_VALUE_BYTES,
        },
        &mut context,
        |(grant, _), event| match event {
            PhysicalReadAdmissionV1::KeyProbe | PhysicalReadAdmissionV1::ValueSizeProbe => grant
                .charge_steps(1)
                .map_err(super::super::super::source_error),
            PhysicalReadAdmissionV1::Value {
                key_bytes,
                value_bytes,
            } => {
                grant
                    .charge_bytes(
                        key_bytes
                            .checked_add(value_bytes)
                            .ok_or(Error::BudgetExceeded)?,
                    )
                    .map_err(super::super::super::source_error)?;
                grant
                    .charge_steps(2)
                    .map_err(super::super::super::source_error)
            }
        },
        |(_, rows), key, raw| {
            rows.push(frozen_source::CapturedJournalRow {
                key: key.to_vec(),
                value: raw.to_vec(),
            });
            Ok(())
        },
    )?;
    Ok(frozen_source::CapturedExternalPrefix {
        root,
        rows: context.1,
    })
}
