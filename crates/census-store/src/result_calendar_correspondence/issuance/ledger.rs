use super::super::*;
use crate::{Receipt, StoreSnapshot};
use census_domain::model::StoreIdentity;
mod storage;
pub(in super::super) use storage::load;

pub(in super::super) struct Ledger {
    pub(in super::super) proof: wire::Proof,
    pub(in super::super) store: StoreIdentity,
    pub(in super::super) initial: ResultCalendarInventoryV1,
    pub(in super::super) inventory: ResultCalendarInventoryV1,
    pub(in super::super) records: Vec<ResultCalendarReviewedRecordV1>,
    pub(in super::super) receipts: Vec<Receipt>,
    pub(in super::super) contexts: Vec<Vec<super::super::source::Material>>,
    pub(in super::super) active_indices: Vec<usize>,
    pub(in super::super) checked_roots: Vec<super::super::source::checked::Root>,
}

impl Ledger {
    pub(in super::super) fn validate(
        &mut self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<()> {
        grant
            .charge_bytes(
                self.records
                    .len()
                    .checked_mul(std::mem::size_of::<usize>())
                    .ok_or(Error::BudgetExceeded)?,
            )
            .map_err(grant_error)?;
        let mut active = Vec::new();
        active
            .try_reserve_exact(self.records.len())
            .map_err(|_| Error::BudgetExceeded)?;
        {
            let projection = self.projection(grant)?;
            projection.active_record_indices().try_for_each(|index| {
                if index >= self.records.len() {
                    return Err(Error::Corrupt("active record index").into());
                }
                active.push(index);
                Ok::<_, StoreError>(())
            })?;
        }
        self.active_indices = active;
        Ok(())
    }

    pub(in super::super) fn active_records(
        &self,
    ) -> impl Iterator<Item = &ResultCalendarReviewedRecordV1> {
        self.records
            .iter()
            .enumerate()
            .filter(|(index, _)| self.active_indices.binary_search(index).is_ok())
            .map(|(_, record)| record)
    }

    pub(in super::super) fn projection<'a>(
        &'a self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<ResultCalendarProjectionV1<'a>> {
        grant
            .charge_bytes(
                self.receipts
                    .len()
                    .checked_mul(std::mem::size_of::<ResultCalendarReceiptBindingV1>())
                    .ok_or(Error::BudgetExceeded)?,
            )
            .map_err(grant_error)?;
        let mut receipts = Vec::new();
        receipts
            .try_reserve_exact(self.receipts.len())
            .map_err(|_| Error::BudgetExceeded)?;
        self.records.iter().zip(&self.receipts).try_for_each(
            |(record, receipt)| -> StoreResult<()> {
                if receipt.operation != wire::issuance_operation(&record.command)
                    || receipt.digest != record.record_digest.as_str()
                    || receipt.at != record.committed_at.to_rfc3339()
                    || receipt.appended != 0
                    || receipt.credited
                {
                    return Err(Error::Corrupt("issuance receipt binding").into());
                }
                receipts.push(ResultCalendarReceiptBindingV1 {
                    operation: record.command.operation.clone(),
                    record_digest: record.record_digest.clone(),
                });
                Ok(())
            },
        )?;
        replay_result_calendar_records_v1(
            &self.store,
            &self.initial,
            &self.records,
            &receipts,
            &self.inventory,
            grant,
        )
        .map_err(|error| Error::Domain(error).into())
    }
}
pub(in super::super) fn retained(
    view: &StoreSnapshot<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Ledger> {
    view.with_roster_source_snapshot_v1(|snapshot, meta, journal, receipts| {
        let format = read::snapshot_format(snapshot, meta, grant)?;
        let proof = read::load(snapshot, meta, journal, receipts, &format, grant)?
            .ok_or(Error::EnrollmentRequired)?;
        load(snapshot, meta, journal, receipts, proof, grant)
    })
}

pub(in super::super) fn from_view(
    view: &StoreSnapshot<'_>,
    attachments: &crate::CurrentEvidenceSubjectAttachmentReferencesV1<'_, '_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Ledger> {
    if !std::ptr::eq(view, attachments.snapshot())
        || view.sequence() != attachments.snapshot_sequence()
    {
        return Err(Error::InputMoved.into());
    }
    let originals = attachments.reviewed_source_v1(grant)?;
    let accepted = attachments.accepted_snapshot(grant)?;
    if view.attachment_public_head()? != *attachments.accepted_public_head() {
        return Err(Error::InputMoved.into());
    }
    let mut ledger = retained(view, grant)?;
    if &ledger.store != attachments.store_identity()
        || accepted.store_identity() != &ledger.store
        || accepted.snapshot_sequence() != view.sequence()
    {
        return Err(Error::InputMoved.into());
    }
    ledger
        .records
        .iter()
        .zip(&ledger.contexts)
        .try_for_each(|(record, context)| {
            super::super::source::checked::retain(
                view,
                &record.command.source,
                context,
                attachments,
                &mut ledger.checked_roots,
                grant,
            )?;
            super::super::source::require_accepted_head(
                &record.command.accepted_head,
                accepted,
                grant,
            )?;
            super::super::source::require_historical(
                &super::super::source::checked::Source::new(view, &ledger.checked_roots),
                &record.command.source,
                context,
                &originals,
                grant,
            )
        })?;
    Ok(ledger)
}
