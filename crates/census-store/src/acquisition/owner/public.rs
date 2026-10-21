use super::*;

impl CaptureWriterV1 {
    pub fn public_resource_capture_digest(&self, store: &Store) -> StoreResult<AcquisitionDigest> {
        self.verify(store)?;
        let declaration = super::super::qualification::declaration(
            &store.db.snapshot(),
            &store.meta,
            &store.journal,
            &store.store_format()?,
        )?
        .ok_or(CaptureError::CaptureCapabilityMissing)?;
        if declaration.protocol() != CaptureProtocolVersion::V2 {
            return Err(CaptureError::CaptureCapabilityNamespaceUnsupported.into());
        }
        crate::acquisition::public_resources::codec::object_digest(&declaration)
    }

    pub fn public_resource_owner(
        &self,
        store: &Store,
        site: UsJurisdiction,
    ) -> StoreResult<PublicResourceCaptureOwnerV1> {
        let _writes = store.lock_appends();
        self.verify(store)?;
        let declaration = read::declaration(
            &store.db.snapshot(),
            &store.meta,
            &store.journal,
            &store.receipts,
            &store.store_format()?,
            &mut AcquisitionReadBudget::for_work(),
        )?
        .ok_or(CaptureError::CaptureCapabilityNamespaceUnsupported)?;
        let scope = scope(&declaration, site)?;
        Ok(PublicResourceCaptureOwnerV1 {
            writer: self.clone(),
            scope,
            declaration_digest: declaration.digest()?,
        })
    }
    pub fn continuation_for_public_resource(
        &self,
        store: &Store,
        owner: &PublicResourceCaptureOwnerV1,
        work: &AcquisitionWorkV1,
    ) -> StoreResult<RetryOwnerGrant> {
        let _writes = store.lock_appends();
        self.verify(store)?;
        CaptureOwnerRef::from(owner).verify_candidate(store, work)?;
        Ok(RetryOwnerGrant {
            owner: CaptureOwner::Public(owner.clone()),
            slot: work.slot().id().map_err(CaptureError::Domain)?,
        })
    }
}

impl PublicResourceCaptureOwnerV1 {
    pub(in crate::acquisition) fn matches_work(&self, work: &AcquisitionWorkV1) -> StoreResult<()> {
        let slot = work.slot();
        let scope = &self.scope;
        if slot.store_identity() != self.writer.store_identity()
            || slot.owner_key() != scope.owner_key()
            || slot.source_site() != scope.jurisdiction()
            || slot.historical_year().is_some()
            || slot.source_meet_id().is_some()
            || slot.public_resource().is_none()
            || work.public_resource_obligation().is_none()
            || work.run() != scope.run()
            || work.run_manifest_digest() != scope.run_manifest_digest()
            || work.history() != scope.history()
            || work.source_admission() != scope.source_admission()
            || work.cache_intent() != scope.cache_intent()
        {
            return Err(CaptureError::ObligationConflict.into());
        }
        Ok(())
    }
    pub fn for_root(
        &self,
        store: &Store,
        kind: PublicResourceRootKindV1,
    ) -> StoreResult<AcquisitionWorkV1> {
        let _writes = store.lock_appends();
        let declaration = self.declaration(store, &mut AcquisitionReadBudget::for_work())?;
        let owner = declaration.binding.owner(self.scope.jurisdiction())?;
        let request = match kind {
            PublicResourceRootKindV1::MileSplitTeamDirectory => &owner.directory_request,
            PublicResourceRootKindV1::Robots => &owner.robots_request,
        };
        Ok(PublicResourceFrontierEntryV1::new(
            &declaration,
            self.scope.jurisdiction(),
            kind,
            request.clone(),
        )?
        .work)
    }
    pub fn scope(&self) -> &PublicResourceCaptureScopeV1 {
        &self.scope
    }
    pub(in crate::acquisition) fn declaration(
        &self,
        store: &Store,
        budget: &mut AcquisitionReadBudget,
    ) -> StoreResult<PublicResourceDeclarationV1> {
        self.writer.verify(store)?;
        let snapshot = store.db.snapshot();
        let format = crate::format::read_capture_format(&snapshot, &store.meta, budget)?;
        let declaration = read::declaration(
            &snapshot,
            &store.meta,
            &store.journal,
            &store.receipts,
            &format,
            budget,
        )?
        .ok_or(CaptureError::CaptureCapabilityNamespaceUnsupported)?;
        self.validate_declaration(&declaration)?;
        Ok(declaration)
    }
    pub(in crate::acquisition) fn validate_declaration(
        &self,
        declaration: &PublicResourceDeclarationV1,
    ) -> StoreResult<()> {
        if declaration.digest()? != self.declaration_digest
            || scope(declaration, self.scope.jurisdiction())? != self.scope
        {
            return Err(CaptureError::ObligationConflict.into());
        }
        Ok(())
    }
}

fn scope(
    declaration: &PublicResourceDeclarationV1,
    site: UsJurisdiction,
) -> StoreResult<PublicResourceCaptureScopeV1> {
    let binding = &declaration.binding;
    let owner = binding.owner(site)?;
    PublicResourceCaptureScopeV1::new(
        &owner.owner_key,
        site,
        binding.run,
        binding.run_manifest_digest.clone(),
        owner.history,
        binding.plan_digest()?,
        owner.cache_intent,
    )
    .map_err(|error| CaptureError::Domain(error).into())
}
