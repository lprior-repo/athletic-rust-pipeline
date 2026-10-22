use super::{
    codec, refused, Entry, PreparedPublicRosterChildV1, PublicDirectoryRosterAssessmentV1,
};
use crate::acquisition::{codec as acquisition, facts::SlotBinding, owner::CaptureOwner};
use crate::{AcquisitionReadBudget, OwnedAcquisitionWork, Receipt, Store, StoreResult};
use census_domain::model::*;
use fjall::Readable;

impl Store {
    pub fn prepare_public_roster_child(
        &self,
        assessment: &impl PublicDirectoryRosterAssessmentV1,
    ) -> StoreResult<PreparedPublicRosterChildV1> {
        let _writes = self.lock_appends();
        let evidence = assessment.evidence();
        let mut budget = AcquisitionReadBudget::for_work();
        evidence.validate_locked(self, &mut budget)?;
        let declaration = evidence.owner.declaration(self, &mut budget)?;
        let entry = Entry::assessed(&declaration, assessment)?;
        if entry.parent_work(&declaration)? != evidence.work {
            return Err(refused("public roster child assessment parent differs"));
        }
        Ok(PreparedPublicRosterChildV1 {
            owner: evidence.owner.clone(),
            entry,
        })
    }

    pub fn register_public_roster_child(
        &self,
        child: PreparedPublicRosterChildV1,
    ) -> StoreResult<OwnedAcquisitionWork> {
        let _writes = self.lock_appends();
        let mut budget = AcquisitionReadBudget::for_work();
        let declaration = child.owner.declaration(self, &mut budget)?;
        child.entry.validate(&declaration)?;
        child.owner.matches_work(&child.entry.work)?;
        if !repeated(self, &declaration, &child.entry, &mut budget)? {
            admit(self, &declaration, &child.entry, &mut budget)?;
        }
        let id = child.entry.work.id().map_err(crate::CaptureError::Domain)?;
        Ok(OwnedAcquisitionWork {
            owner: CaptureOwner::Public(child.owner),
            work: child.entry.work,
            id,
        })
    }
}

fn repeated(
    store: &Store,
    declaration: &super::PublicResourceDeclarationV1,
    entry: &Entry,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<bool> {
    let snapshot = store.db.snapshot();
    let id = entry.work.id().map_err(crate::CaptureError::Domain)?;
    let slot = entry
        .work
        .slot()
        .id()
        .map_err(crate::CaptureError::Domain)?;
    let protocol = CaptureProtocolVersion::V2;
    let binding: Option<SlotBinding> = acquisition::get_fact(
        &snapshot,
        &store.journal,
        &format!("slot/{slot}/binding"),
        budget,
        protocol,
    )?;
    let work: Option<AcquisitionWorkV1> = acquisition::get(
        &snapshot,
        &store.journal,
        &format!("work/{id}/identity"),
        budget,
        protocol,
    )?;
    match (binding, work) {
        (Some(binding), Some(work))
            if binding.version == protocol
                && binding.work_id == id
                && binding.slot == *entry.work.slot()
                && work == entry.work =>
        {
            repeat_child(store, &snapshot, declaration, entry, &id, budget)
        }
        (None, None) => {
            if snapshot
                .prefix(
                    &store.journal,
                    Store::journal_key(protocol.phase(), &format!("work/{id}/")),
                )
                .next()
                .is_some()
            {
                return Err(crate::CaptureError::SlotWorkMismatch.into());
            }
            Ok(false)
        }
        (Some(binding), _) if binding.work_id != id => {
            Err(crate::CaptureError::ObligationConflict.into())
        }
        _ => Err(crate::CaptureError::SlotWorkMismatch.into()),
    }
}

fn admit(
    store: &Store,
    declaration: &super::PublicResourceDeclarationV1,
    entry: &Entry,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<()> {
    let parent = entry.parent_work(declaration)?;
    if entry.parent.attempt().work_id() != &parent.id().map_err(crate::CaptureError::Domain)? {
        return Err(refused("public roster child foreign parent work"));
    }
    let retained = store
        .snapshot_locked()
        .read_retained_acquisition_v1(&entry.parent, budget)?;
    entry.require_retained(&retained)?;
    let snapshot = store.db.snapshot();
    let key = entry.key()?;
    let operation = entry.operation()?;
    for (table, key) in [
        (&store.journal, key.as_str()),
        (&store.receipts, operation.as_str()),
    ] {
        if snapshot
            .contains_key(table, key.as_bytes())
            .map_err(|source| crate::StoreError::Read { source })?
        {
            return Err(refused(
                "public roster child authority already present without registration",
            ));
        }
    }
    super::startup::require_capacity(&snapshot, &store.journal, entry)?;
    let receipt = Receipt {
        operation,
        digest: codec::object_digest(entry)?.as_str().into(),
        at: chrono::Utc::now().to_rfc3339(),
        appended: 0,
        credited: false,
    };
    entry.validate_receipt(declaration, &receipt)?;
    let mut batch = store.db.batch();
    crate::acquisition::transition::stage_identity(
        store,
        &mut batch,
        &entry.work,
        CaptureProtocolVersion::V2,
    )?;
    batch.insert(&store.journal, key.as_bytes(), codec::encode(entry)?);
    batch.insert(
        &store.receipts,
        receipt.operation.as_bytes(),
        codec::encode(&receipt)?,
    );
    acquisition::commit(batch)?;
    Ok(())
}

fn repeat_child(
    store: &Store,
    snapshot: &fjall::Snapshot,
    declaration: &super::PublicResourceDeclarationV1,
    entry: &Entry,
    id: &AcquisitionWorkId,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<bool> {
    let held = super::read::validate_entry(
        snapshot,
        &store.journal,
        &store.receipts,
        entry.key()?.as_bytes(),
        declaration,
        budget,
    )?;
    if &held != entry {
        return Err(refused("public roster child witness rebound"));
    }
    crate::acquisition::read::read_work(
        snapshot,
        &store.journal,
        &store.receipts,
        id,
        budget,
        CaptureProtocolVersion::V2,
    )?;
    acquisition::repeat(store)?;
    Ok(true)
}
