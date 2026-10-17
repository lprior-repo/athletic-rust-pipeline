use super::*;
use super::super::super::{codec, format, RosterStartupValidationBudgetV1};
use crate::read::JournalNamespace;
use crate::{PhysicalReadAdmissionV1, ReadBudget, StoreError};
use fjall::Readable;
use std::collections::BTreeMap;

pub(super) fn read(view: &StoreSnapshot<'_>, grant: &mut AttachmentReplayGrantV1) -> StoreResult<CurrentRosterReviewedClaimsV1> {
    let continuation = grant.checkpoint().map_err(super::super::super::source_error)?;
    let (metadata, records, receipts) = view.with_roster_source_snapshot_v1(|snapshot, meta, journal, receipts| {
        lifecycle(snapshot, meta, grant)?;
        let mut budget = RosterStartupValidationBudgetV1::new();
        let metadata = format::protected_rows(snapshot, meta, &mut budget, Some(&mut *grant))?;
        let records = format::protected_rows(snapshot, journal, &mut budget, Some(&mut *grant))?;
        let receipts = format::scan(snapshot, receipts, super::super::super::OPERATION_PREFIX, &mut budget, Some(&mut *grant))?;
        Ok((metadata, records, receipts))
    })?;
    grant.charge_bytes(1024).map_err(super::super::super::source_error)?;
    grant.charge_steps(8).map_err(super::super::super::source_error)?;
    let store = view.store_identity()?;
    format::validate_enrollment(&metadata, &records, &receipts, &store)?;
    let rows = super::super::super::inspection::inventory::collect(view, grant)?;
    let physical = rows.into_iter().map(|row| { let (table, key, value) = row.into_parts(); PhysicalRow { table, key, value } }).collect();
    grant.charge_bytes(1024).map_err(super::super::super::source_error)?;
    grant.charge_steps(8).map_err(super::super::super::source_error)?;
    let current_input_token = view.attachment_input_revision_bytes()?;
    let mut archive = Archive { schema: "roster-reviewed-source-publication-closure-v1".into(), store,
        selected_snapshot_sequence: view.sequence(), source_store_lifecycle: super::super::super::LIFECYCLE,
        current_input_token, metadata, records, receipts, physical, external: Vec::new() };
    let roots = roots(&archive, grant)?;
    for root in roots {
        continuation.require(grant).map_err(super::super::super::source_error)?;
        archive.external.push(capture(view, root, grant)?);
    }
    let claims = reconstruct::claims(&archive, grant)?;
    continuation.require(grant).map_err(super::super::super::source_error)?;
    Ok(CurrentRosterReviewedClaimsV1 { archive, claims, continuation })
}

fn lifecycle(snapshot: &fjall::Snapshot, meta: &fjall::Keyspace, grant: &mut AttachmentReplayGrantV1) -> StoreResult<()> {
    for (key, expected) in [("schema:store_version", Some(b"2".as_slice())), ("schema:key_format", Some(b"1".as_slice())),
        ("schema:lifecycle_revision", Some(b"8".as_slice())), ("schema:migrating_to", None)] {
        grant.charge_steps(2).map_err(super::super::super::source_error)?;
        grant.charge_bytes(32).map_err(super::super::super::source_error)?;
        let size = snapshot.size_of(meta, key).map_err(|source| StoreError::Read { source })?;
        if size.is_some_and(|size| size > 32) { return Err(Error::BudgetExceeded.into()); }
        let value = snapshot.get(meta, key).map_err(|source| StoreError::Read { source })?;
        if value.as_ref().map(|raw| raw.as_ref()) != expected { return Err(Error::EnrollmentRequired.into()); }
    }
    Ok(())
}

fn roots(archive: &Archive, grant: &mut AttachmentReplayGrantV1) -> StoreResult<Vec<crate::RetainedCaptureRootV1>> {
    let objects = reconstruct::objects(archive, grant)?;
    let mut roots = BTreeMap::new();
    for (key, raw) in &archive.records {
        if !key.starts_with(super::super::RECORD_PREFIX.as_bytes()) { continue; }
        grant.charge_bytes(raw.len()).map_err(super::super::super::source_error)?;
        grant.charge_steps(1).map_err(super::super::super::source_error)?;
        let record: ReviewRecord = codec::decode(raw)?;
        let proposal = super::super::proposal(&record.basis.proposal)?;
        let admission = reconstruct::admission(&proposal, &objects, grant)?;
        let key = (admission.root.schema.phase(), admission.root.key.clone());
        if let Some(prior) = roots.insert(key, admission.root.clone()) {
            if prior != admission.root { return Err(Error::Corrupt("review root disagreement").into()); }
        }
    }
    Ok(roots.into_values().collect())
}

fn capture(view: &StoreSnapshot<'_>, root: crate::RetainedCaptureRootV1, grant: &mut AttachmentReplayGrantV1) -> StoreResult<frozen_source::CapturedExternalPrefix> {
    let remaining = census_domain::public_identity_history::ReplayLimits::SMALL.total_text_bytes
        .checked_sub(grant.usage().encoded_bytes()).ok_or(Error::BudgetExceeded)?;
    let mut context = (grant, Vec::new());
    view.scan_journal_namespace_preadmitted(JournalNamespace::new(root.schema.phase(), &root.key)?, ReadBudget {
        rows: 6, bytes: u64::try_from(remaining).map_err(|_| Error::BudgetExceeded)?, record_bytes: crate::MAX_JOURNAL_VALUE_BYTES,
    }, &mut context, |(grant, _), event| match event {
        PhysicalReadAdmissionV1::KeyProbe | PhysicalReadAdmissionV1::ValueSizeProbe => {
            grant.charge_steps(1).map_err(super::super::super::source_error)
        }
        PhysicalReadAdmissionV1::Value { key_bytes, value_bytes } => {
            grant.charge_bytes(key_bytes.checked_add(value_bytes).ok_or(Error::BudgetExceeded)?).map_err(super::super::super::source_error)?;
            grant.charge_steps(2).map_err(super::super::super::source_error)
        }
    }, |(_, rows), key, raw| {
        rows.push(frozen_source::CapturedJournalRow { key: key.to_vec(), value: raw.to_vec() });
        Ok(())
    })?;
    Ok(frozen_source::CapturedExternalPrefix { root, rows: context.1 })
}
