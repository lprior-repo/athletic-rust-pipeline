use super::{
    keys,
    material::{self, Material},
    EvidenceSubjectAttachmentStoreError as Error, OPERATION_PREFIX, PHASE,
};
use crate::read::JournalNamespaceInventory;
use crate::{Receipt, StoreError, StoreResult, StoreSnapshot};
use census_domain::evidence_subject_attachment::{
    self as domain, AttachmentDigestV1, AttachmentInventoryV1, AttachmentObjectReferenceV1,
    AttachmentReceiptBindingV1, AttachmentReplayGrantV1, CheckedReviewedReferenceRecordV1,
    CheckedReviewedReferenceRecordsV1, ReviewedReferenceRecordV1,
};
use census_domain::model::StoreIdentity;
use census_domain::public_identity_history::ReplayLimits;

mod context;
pub(super) use context::declared;
pub(super) mod fences;
mod read;
mod receipts;
mod refresh;
pub(super) mod wire;

pub(super) struct Ledger<'s> {
    pub(super) records: CheckedReviewedReferenceRecordsV1,
    pub(super) receipts: Vec<Receipt>,
    pub(super) introductions: Vec<Vec<Material>>,
    pub(super) input_refs: Vec<wire::EntryInputs>,
    pub(super) inventory: AttachmentInventoryV1,
    pub(super) physical: JournalNamespaceInventory<'s>,
}

impl<'s> Ledger<'s> {
    pub(super) fn load(
        view: &StoreSnapshot<'s>,
        store: &StoreIdentity,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Self> {
        grant.ensure_available().map_err(Error::Domain)?;
        let result = read::load(view, store, grant);
        if result.is_err() {
            grant.poison();
        }
        result
    }

    pub(super) fn refresh(
        &mut self,
        store: &crate::Store,
        view: &StoreSnapshot<'_>,
        identity: &StoreIdentity,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<bool> {
        grant.ensure_available().map_err(Error::Domain)?;
        let result = refresh::refresh(self, store, view, identity, grant);
        if result.is_err() {
            grant.poison();
        }
        result
    }

    pub(super) fn find_operation(
        &self,
        operation: &str,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Option<usize>> {
        for (index, record) in self.records.iter().enumerate() {
            grant.charge_steps(1).map_err(Error::Domain)?;
            if record.command.operation.as_str() == operation {
                return Ok(Some(index));
            }
        }
        Ok(None)
    }

    pub(super) fn find_object(
        &self,
        digest: &AttachmentDigestV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Option<&Material>> {
        context::find(&self.introductions, digest, grant)
    }

    pub(super) fn append_proposed(
        &mut self,
        view: &StoreSnapshot<'_>,
        record: CheckedReviewedReferenceRecordV1,
        introduced: Vec<Material>,
        inputs: wire::EntryInputs,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<AttachmentInventoryV1> {
        context::accept(view, &self.introductions, &introduced, &record, grant)?;
        material::allocate(&mut self.introductions)?;
        material::allocate(&mut self.input_refs)?;
        self.input_refs.push(inputs);
        self.records
            .push_with_grant(record, grant)
            .map_err(Error::Domain)?;
        self.introductions.push(introduced);
        let inventory =
            domain::calculate_reviewed_reference_inventory_with_grant(&self.records, grant)
                .map_err(Error::Domain)?;
        let previous = std::mem::replace(&mut self.inventory, inventory);
        let record = self
            .records
            .last()
            .ok_or(Error::Corrupt("proposed record absent"))?;
        let inputs = self
            .input_refs
            .last()
            .ok_or(Error::Corrupt("proposed inputs absent"))?;
        fences::verify(self, record, inputs, grant)?;
        Ok(previous)
    }

    pub(super) fn object(
        &self,
        digest: &AttachmentDigestV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<&Material> {
        context::find(&self.introductions, digest, grant)?
            .ok_or_else(|| Error::Corrupt("missing retained context object").into())
    }

    pub(super) fn validate_projection(
        &self,
        store: &StoreIdentity,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<()> {
        self.projection(store, grant).map(|_| ())
    }

    pub(super) fn projection<'a>(
        &'a self,
        store: &StoreIdentity,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<domain::ReviewedReferenceProjectionV1<'a>> {
        let mut receipts = Vec::new();
        for record in self.records.iter() {
            grant.charge_steps(1).map_err(Error::Domain)?;
            material::allocate(&mut receipts)?;
            receipts.push(AttachmentReceiptBindingV1 {
                operation: record.command.operation.clone(),
                record_digest: record.record_digest.clone(),
            });
        }
        domain::project_checked_reviewed_references_with_grant(
            store,
            &self.records,
            &receipts,
            &self.inventory,
            grant,
        )
        .map_err(|source| Error::Domain(source).into())
    }
}
