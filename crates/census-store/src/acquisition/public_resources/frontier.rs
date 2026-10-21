use super::declaration::{PublicResourceDeclarationV1, PublicResourceOwnerBindingV1};
use super::{codec, refused};
use crate::acquisition::CaptureError;
use crate::StoreResult;
use census_domain::model::*;
use census_domain::UsJurisdiction;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicResourceRootKindV1 {
    MileSplitTeamDirectory,
    Robots,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PublicResourceFrontierEntryV1 {
    pub(crate) version: u8,
    pub(crate) declaration_digest: AcquisitionDigest,
    pub(crate) plan_digest: AcquisitionDigest,
    pub(crate) kind: PublicResourceRootKindV1,
    pub(crate) work: AcquisitionWorkV1,
}

impl PublicResourceFrontierEntryV1 {
    pub(crate) fn new(
        declaration: &PublicResourceDeclarationV1,
        site: UsJurisdiction,
        kind: PublicResourceRootKindV1,
        request: RequestRepresentationV1,
    ) -> StoreResult<Self> {
        declaration.validate_shape()?;
        let binding = &declaration.binding;
        let owner = binding.owner(site)?;
        owner.validate_requests()?;
        if &request != bound_request(owner, kind) {
            return Err(refused("public resource frontier request differs"));
        }
        let plan_digest = binding.plan_digest()?;
        let scope = PublicResourceCaptureScopeV1::new(
            &owner.owner_key,
            site,
            binding.run,
            binding.run_manifest_digest.clone(),
            owner.history,
            plan_digest.clone(),
            owner.cache_intent,
        )
        .map_err(CaptureError::Domain)?;
        let as_of = ProfileDate::parse(&owner.history.as_of().to_string())
            .map_err(|_| refused("public resource frontier as of"))?;
        let work = AcquisitionWorkV1::for_public_resource(
            binding.store.clone(),
            &scope,
            site,
            PublicResourceObligationV1::new(resource(site, kind)?, as_of),
            request,
        )
        .map_err(CaptureError::Domain)?;
        let entry = Self {
            version: 1,
            declaration_digest: declaration.digest()?,
            plan_digest,
            kind,
            work,
        };
        entry.validate(declaration)?;
        Ok(entry)
    }

    pub(crate) fn id(&self) -> StoreResult<AcquisitionDigest> {
        codec::object_digest(&("public-resource-frontier-entry-v1", self))
    }

    pub(crate) fn validate(&self, declaration: &PublicResourceDeclarationV1) -> StoreResult<()> {
        declaration.validate_shape()?;
        let binding = &declaration.binding;
        let slot = self.work.slot();
        let owner = binding.owner(slot.source_site())?;
        owner.validate_requests()?;
        let plan = binding.plan_digest()?;
        if self.version != 1
            || self.declaration_digest != declaration.digest()?
            || self.plan_digest != plan
            || slot.store_identity() != &binding.store
            || slot.owner_key() != owner.owner_key
            || slot.historical_year().is_some()
            || slot.source_meet_id().is_some()
            || slot.public_resource() != Some(&resource(owner.site, self.kind)?)
        {
            return Err(refused("public resource frontier identity"));
        }
        self.validate_work(declaration, owner, &plan)
    }

    fn validate_work(
        &self,
        declaration: &PublicResourceDeclarationV1,
        owner: &PublicResourceOwnerBindingV1,
        plan: &AcquisitionDigest,
    ) -> StoreResult<()> {
        let binding = &declaration.binding;
        let obligation = self
            .work
            .public_resource_obligation()
            .ok_or_else(|| refused("public resource frontier obligation absent"))?;
        let admission =
            SourceAdmissionV1::public_resources(plan.as_str()).map_err(CaptureError::Domain)?;
        if self.work.run() != binding.run
            || self.work.run_manifest_digest() != &binding.run_manifest_digest
            || self.work.history() != owner.history
            || self.work.cache_intent() != owner.cache_intent
            || self.work.source_admission() != &admission
            || self.work.request() != bound_request(owner, self.kind)
            || obligation.resource() != &resource(owner.site, self.kind)?
            || obligation.as_of().as_str() != owner.history.as_of().to_string()
        {
            return Err(refused("public resource frontier work binding"));
        }
        Ok(())
    }
}

fn bound_request(
    owner: &PublicResourceOwnerBindingV1,
    kind: PublicResourceRootKindV1,
) -> &RequestRepresentationV1 {
    match kind {
        PublicResourceRootKindV1::MileSplitTeamDirectory => &owner.directory_request,
        PublicResourceRootKindV1::Robots => &owner.robots_request,
    }
}

fn resource(
    site: UsJurisdiction,
    kind: PublicResourceRootKindV1,
) -> StoreResult<PublicSourceResourceV1> {
    match kind {
        PublicResourceRootKindV1::MileSplitTeamDirectory => {
            Ok(PublicSourceResourceV1::milesplit_team_directory())
        }
        PublicResourceRootKindV1::Robots => PublicSourceResourceV1::robots(&format!(
            "https://{}.milesplit.com",
            site.code().to_ascii_lowercase()
        ))
        .map_err(|error| CaptureError::Domain(error).into()),
    }
}

pub(crate) fn entry_id(
    declaration: &PublicResourceDeclarationV1,
    work: &AcquisitionWorkV1,
) -> StoreResult<AcquisitionDigest> {
    #[derive(Serialize)]
    struct Entry<'a> {
        version: u8,
        declaration_digest: AcquisitionDigest,
        plan_digest: AcquisitionDigest,
        kind: PublicResourceRootKindV1,
        work: &'a AcquisitionWorkV1,
    }
    let kind = root_kind(work)?;
    let view = Entry {
        version: 1,
        declaration_digest: declaration.digest()?,
        plan_digest: declaration.binding.plan_digest()?,
        kind,
        work,
    };
    codec::object_digest(&("public-resource-frontier-entry-v1", &view))
}

pub(crate) fn root_kind(work: &AcquisitionWorkV1) -> StoreResult<PublicResourceRootKindV1> {
    match work
        .slot()
        .public_resource()
        .map(PublicSourceResourceV1::kind)
    {
        Some(PublicSourceResourceKindV1::MileSplitTeamDirectory) => {
            Ok(PublicResourceRootKindV1::MileSplitTeamDirectory)
        }
        Some(PublicSourceResourceKindV1::Robots) => Ok(PublicResourceRootKindV1::Robots),
        _ => Err(refused("public resource frontier root role")),
    }
}
