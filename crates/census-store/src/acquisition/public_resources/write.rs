use super::{codec, format, read, refused, PublicResourceDeclarationBindingV1};
use super::{PublicResourceDeclarationV1, DECLARATION};
use crate::acquisition::{AcquisitionReadBudget, CaptureWriterV1};
use crate::{Application, Store, StoreResult};
use chrono::{DateTime, Utc};

impl Store {
    pub fn declare_public_resource_capture(
        &self,
        writer: &CaptureWriterV1,
        binding: PublicResourceDeclarationBindingV1,
        admitted_at: DateTime<Utc>,
    ) -> StoreResult<Application> {
        let _writes = self.lock_appends();
        writer.verify(self)?;
        let snapshot = self.db.snapshot();
        let store_format = self.store_format()?;
        format::validate_format(
            &snapshot,
            &self.meta,
            &self.journal,
            &self.receipts,
            &store_format,
        )?;
        let mut budget = AcquisitionReadBudget::for_work();
        if let Some(held) = read::declaration(
            &snapshot,
            &self.meta,
            &self.journal,
            &self.receipts,
            &store_format,
            &mut budget,
        )? {
            let operation = held.receipt()?.operation;
            let receipt =
                read::record(&snapshot, &self.receipts, operation.as_bytes(), &mut budget)?
                    .ok_or_else(|| refused("public resource declaration receipt absent"))?;
            held.replay(&binding, &receipt)?;
            crate::acquisition::codec::repeat(self)?;
            return Ok(Application::Repeated(receipt));
        }
        let capture = crate::acquisition::qualification::declaration(
            &snapshot,
            &self.meta,
            &self.journal,
            &store_format,
        )?
        .ok_or(crate::CaptureError::CaptureCapabilityMissing)?;
        let manifest = self
            .run_manifest()?
            .ok_or(crate::CaptureError::CaptureRunManifestMissing)?;
        binding
            .selection
            .ensure_store(writer.store_identity())
            .map_err(|_| refused("public resource selected profile store"))?;
        let profile =
            crate::profiles::read::retained(&snapshot, &self.meta, binding.selection.profile_id())?
                .ok_or_else(|| crate::ProfileRegistryError::MissingProfile {
                    id: binding.selection.profile_id().clone(),
                })?;
        binding.validate(&capture, &manifest, &profile)?;
        let declaration = PublicResourceDeclarationV1::new(binding, admitted_at)?;
        let receipt = declaration.receipt()?;
        let mut batch = self.db.batch();
        batch.insert(
            &self.journal,
            DECLARATION.as_bytes(),
            codec::encode(&declaration)?,
        );
        batch.insert(
            &self.receipts,
            receipt.operation.as_bytes(),
            codec::encode(&receipt)?,
        );
        crate::acquisition::codec::commit(batch)?;
        Ok(Application::Written(receipt))
    }
}

pub(in crate::acquisition) fn stage_entry(
    store: &Store,
    batch: &mut fjall::OwnedWriteBatch,
    owner: crate::CaptureOwnerRef<'_>,
    work: &census_domain::model::AcquisitionWorkV1,
) -> StoreResult<()> {
    use fjall::Readable;
    let Some(owner) = owner.public() else {
        return Ok(());
    };
    let declaration = owner.declaration(store, &mut AcquisitionReadBudget::for_work())?;
    let kind = super::frontier::root_kind(work)?;
    let entry = super::PublicResourceFrontierEntryV1::new(
        &declaration,
        owner.scope().jurisdiction(),
        kind,
        work.request().clone(),
    )?;
    if &entry.work != work {
        return Err(crate::CaptureError::ObligationConflict.into());
    }
    let id = entry.id()?;
    let key = format!("{}{id}", super::ENTRY_PREFIX);
    let receipt = crate::Receipt {
        operation: format!("{}{id}", super::FRONTIER_OPERATION_PREFIX),
        digest: codec::object_digest(&entry)?.as_str().to_string(),
        at: Utc::now().to_rfc3339(),
        appended: 0,
        credited: false,
    };
    format::validate_entry_receipt(&declaration, &entry, &receipt)?;
    let snapshot = store.db.snapshot();
    if snapshot
        .contains_key(&store.journal, key.as_bytes())
        .map_err(|source| crate::StoreError::Read { source })?
        || snapshot
            .contains_key(&store.receipts, receipt.operation.as_bytes())
            .map_err(|source| crate::StoreError::Read { source })?
    {
        return Err(refused(
            "public resource frontier already present without registration",
        ));
    }
    batch.insert(&store.journal, key.as_bytes(), codec::encode(&entry)?);
    batch.insert(
        &store.receipts,
        receipt.operation.as_bytes(),
        codec::encode(&receipt)?,
    );
    Ok(())
}
