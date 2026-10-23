use super::PublicRosterProfileAssessmentV1;
use super::{codec, counter, read, refused, Entry, PreparedPublicProfileChildV1};
use crate::acquisition::public_resources::PublicResourceDeclarationV1;
use crate::acquisition::{codec as acquisition, facts::SlotBinding, owner::CaptureOwner};
use crate::{AcquisitionReadBudget, OwnedAcquisitionWork, Receipt, Store, StoreResult};
use census_domain::model::{AcquisitionWorkV1, CaptureProtocolVersion};
use fjall::Readable;

impl Store {
    pub(crate) fn prepare_public_profile_child(
        &self,
        assessment: &impl PublicRosterProfileAssessmentV1,
    ) -> StoreResult<PreparedPublicProfileChildV1> {
        let _writes = self.lock_appends();
        let evidence = assessment.evidence();
        let mut budget = AcquisitionReadBudget::for_work();
        evidence.validate_locked(self, &mut budget)?;
        let declaration = evidence.owner.declaration(self, &mut budget)?;
        let candidate = Entry::assessed(&declaration, assessment)?;
        let entry = match original(self, &declaration, &candidate, &mut budget)? {
            Some(held) => held,
            None => candidate,
        };
        Ok(PreparedPublicProfileChildV1 {
            owner: evidence.owner.clone(),
            entry,
        })
    }

    pub fn register_public_profile_child(
        &self,
        child: PreparedPublicProfileChildV1,
    ) -> StoreResult<OwnedAcquisitionWork> {
        let _writes = self.lock_appends();
        let mut budget = AcquisitionReadBudget::for_work();
        let declaration = child.owner.declaration(self, &mut budget)?;
        child.owner.matches_work(&child.entry.work)?;
        child.entry.validate(&declaration)?;
        let entry = match original(self, &declaration, &child.entry, &mut budget)? {
            Some(held) => {
                acquisition::repeat(self)?;
                held
            }
            None => {
                admit(self, &declaration, &child.entry, &mut budget)?;
                child.entry
            }
        };
        let id = entry.work.id().map_err(crate::CaptureError::Domain)?;
        Ok(OwnedAcquisitionWork {
            owner: CaptureOwner::Public(child.owner),
            work: entry.work,
            id,
        })
    }
}

fn original(
    store: &Store,
    declaration: &PublicResourceDeclarationV1,
    candidate: &Entry,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<Option<Entry>> {
    let snapshot = store.db.snapshot();
    let key = candidate.key();
    if !snapshot
        .contains_key(&store.journal, key.as_bytes())
        .map_err(|source| crate::StoreError::Read { source })?
    {
        return Ok(None);
    }
    let held = read::entry(
        &snapshot,
        &store.meta,
        &store.journal,
        &store.receipts,
        declaration,
        key.as_bytes(),
        budget,
    )?;
    if held.work.slot() != candidate.work.slot() {
        return Err(refused("public profile child corroboration owner differs"));
    }
    Ok(Some(held))
}

fn admit(
    store: &Store,
    declaration: &PublicResourceDeclarationV1,
    entry: &Entry,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<()> {
    let snapshot = store.db.snapshot();
    read::parent(
        &snapshot,
        &store.meta,
        &store.journal,
        &store.receipts,
        declaration,
        entry,
        budget,
    )?;
    require_unregistered(store, entry, budget)?;
    let key = entry.key();
    let receipt = admission_receipt(entry)?;
    if snapshot
        .contains_key(&store.receipts, receipt.operation.as_bytes())
        .map_err(|source| crate::StoreError::Read { source })?
    {
        return Err(refused("public profile child orphan receipt"));
    }
    let (counter_key, counter) = counter::prepare(
        &snapshot,
        &store.meta,
        &store.journal,
        declaration,
        entry,
        budget,
    )?;
    entry.require_receipt(declaration, &receipt)?;
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
    batch.insert(
        &store.meta,
        counter_key.as_bytes(),
        codec::encode(&counter)?,
    );
    acquisition::commit(batch)?;
    Ok(())
}

fn admission_receipt(entry: &Entry) -> StoreResult<Receipt> {
    Ok(Receipt {
        operation: entry.operation()?,
        digest: codec::object_digest(entry)?.as_str().to_owned(),
        at: chrono::Utc::now().to_rfc3339(),
        appended: 0,
        credited: false,
    })
}

fn require_unregistered(
    store: &Store,
    entry: &Entry,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<()> {
    let snapshot = store.db.snapshot();
    let work = &entry.work;
    let id = work.id().map_err(crate::CaptureError::Domain)?;
    let slot = work.slot().id().map_err(crate::CaptureError::Domain)?;
    let protocol = CaptureProtocolVersion::V2;
    let binding: Option<SlotBinding> = acquisition::get_fact(
        &snapshot,
        &store.journal,
        &format!("slot/{slot}/binding"),
        budget,
        protocol,
    )?;
    let held: Option<AcquisitionWorkV1> = acquisition::get(
        &snapshot,
        &store.journal,
        &format!("work/{id}/identity"),
        budget,
        protocol,
    )?;
    if binding.is_some()
        || held.is_some()
        || snapshot
            .prefix(
                &store.journal,
                Store::journal_key(protocol.phase(), &format!("work/{id}/")),
            )
            .next()
            .is_some()
    {
        return Err(refused(
            "public profile child work exists without owner entry",
        ));
    }
    Ok(())
}
