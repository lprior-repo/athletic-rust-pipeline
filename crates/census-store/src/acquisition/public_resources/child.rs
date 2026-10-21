use super::{codec, refused, PublicDirectoryRosterAssessmentV1};
use super::{PublicDirectorySemanticContractV1, PublicResourceDeclarationV1};
use crate::{PublicResourceCaptureOwnerV1, Receipt, StoreResult};
use census_domain::model::*;
use census_domain::roster_affiliation_authority::SourceRowLocator;
use serde::{Deserialize, Serialize};

mod read;
pub(crate) mod startup;
mod write;
pub(in crate::acquisition) use read::{for_work, validate_entry};

pub(crate) const MAX_TEAMS: usize = 20_000;

pub struct PreparedPublicRosterChildV1 {
    owner: PublicResourceCaptureOwnerV1,
    entry: Entry,
}

impl PreparedPublicRosterChildV1 {
    pub fn work(&self) -> &AcquisitionWorkV1 {
        &self.entry.work
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Entry {
    version: u8,
    declaration_digest: AcquisitionDigest,
    plan_digest: AcquisitionDigest,
    pub(super) work: AcquisitionWorkV1,
    pub(super) parent: AcquisitionOccurrenceRefV1,
    pub(super) body_digest: AcquisitionDigest,
    pub(super) body_bytes: u64,
    pub(super) metadata_digest: AcquisitionDigest,
    semantic_contract: PublicDirectorySemanticContractV1,
    pub(super) kernel_digest: AcquisitionDigest,
    pub(super) team_id: String,
    pub(super) team_url: String,
    pub(super) parsed_teams: usize,
    pub(super) row: SourceRowLocator,
    pub(super) name: SourceRowLocator,
    pub(super) city: SourceRowLocator,
    pub(super) anchor: SourceRowLocator,
    pub(super) row_digest: AcquisitionDigest,
}

impl Entry {
    fn parent_work(
        &self,
        declaration: &PublicResourceDeclarationV1,
    ) -> StoreResult<AcquisitionWorkV1> {
        let site = self.work.slot().source_site();
        let root = declaration.binding.owner(site)?;
        Ok(super::PublicResourceFrontierEntryV1::new(
            declaration,
            site,
            super::PublicResourceRootKindV1::MileSplitTeamDirectory,
            root.directory_request.clone(),
        )?
        .work)
    }

    fn require_retained(&self, retained: &crate::VerifiedRetainedOccurrence) -> StoreResult<()> {
        if retained.reference() != &self.parent
            || retained.body_reference() != &self.parent
            || retained.revalidation().is_some()
            || retained.response().status() != 200
            || retained.response().observed_http().is_none()
            || retained.prepared().content_digest() != &self.body_digest
            || retained.prepared().content_bytes() != self.body_bytes
            || retained.prepared().metadata_digest() != &self.metadata_digest
        {
            return Err(refused("public roster child retained parent differs"));
        }
        Ok(())
    }

    fn assessed(
        declaration: &PublicResourceDeclarationV1,
        assessment: &impl PublicDirectoryRosterAssessmentV1,
    ) -> StoreResult<Self> {
        let evidence = assessment.evidence();
        let retained = evidence.retained();
        let entry = Self {
            version: 1,
            declaration_digest: declaration.digest()?,
            plan_digest: declaration.binding.plan_digest()?,
            work: child_work(
                declaration,
                evidence.work().slot().source_site(),
                assessment.team_id(),
                assessment.team_url(),
            )?,
            parent: retained.reference().clone(),
            body_digest: retained.prepared().content_digest().clone(),
            body_bytes: retained.prepared().content_bytes(),
            metadata_digest: retained.prepared().metadata_digest().clone(),
            semantic_contract: assessment.semantic_contract(),
            kernel_digest: assessment.kernel_digest().clone(),
            team_id: assessment.team_id().into(),
            team_url: assessment.team_url().into(),
            parsed_teams: assessment.parsed_teams(),
            row: assessment.row(),
            name: assessment.name(),
            city: assessment.city(),
            anchor: assessment.anchor(),
            row_digest: assessment.row_digest().clone(),
        };
        if assessment.roster_url() != entry.work.request().original_url() {
            return Err(refused("public roster child derived target differs"));
        }
        entry.validate(declaration)?;
        Ok(entry)
    }

    pub(crate) fn validate(&self, declaration: &PublicResourceDeclarationV1) -> StoreResult<()> {
        let site = self.work.slot().source_site();
        if self.version != 1
            || self.declaration_digest != declaration.digest()?
            || self.plan_digest != declaration.binding.plan_digest()?
            || self.work != child_work(declaration, site, &self.team_id, &self.team_url)?
            || self.parsed_teams == 0
            || self.parsed_teams > MAX_TEAMS
            || self.body_bytes == 0
            || self.body_bytes > 8 * 1024 * 1024
        {
            return Err(refused("public roster child binding"));
        }
        let row_end = end(self.row, self.body_bytes)?;
        for part in [self.name, self.city, self.anchor] {
            let part_end = end(part, self.body_bytes)?;
            if part.byte_offset() < self.row.byte_offset() || part_end > row_end {
                return Err(refused("public roster child locator outside row"));
            }
        }
        Ok(())
    }

    pub(crate) fn key(&self) -> StoreResult<String> {
        key(&self.work)
    }

    pub(crate) fn operation(&self) -> StoreResult<String> {
        Ok(format!(
            "{}{}",
            super::CHILD_OPERATION_PREFIX,
            self.work.id().map_err(crate::CaptureError::Domain)?
        ))
    }

    pub(crate) fn validate_receipt(
        &self,
        declaration: &PublicResourceDeclarationV1,
        receipt: &Receipt,
    ) -> StoreResult<()> {
        self.validate(declaration)?;
        let at = chrono::DateTime::parse_from_rfc3339(&receipt.at)
            .map_err(|_| refused("public roster child receipt time"))?;
        if receipt.operation != self.operation()?
            || receipt.digest != codec::object_digest(self)?.as_str()
            || at < declaration.admitted_at
            || receipt.appended != 0
            || receipt.credited
        {
            return Err(refused("public roster child receipt"));
        }
        Ok(())
    }
}

fn end(locator: SourceRowLocator, body: u64) -> StoreResult<usize> {
    let limit = usize::try_from(body).map_err(|_| crate::StoreError::CounterOverflow)?;
    let end = locator
        .byte_offset()
        .checked_add(locator.byte_length())
        .ok_or_else(|| refused("public roster child locator overflow"))?;
    if locator.byte_length() == 0 || end > limit {
        return Err(refused("public roster child locator bounds"));
    }
    Ok(end)
}

pub(crate) fn key(work: &AcquisitionWorkV1) -> StoreResult<String> {
    Ok(format!(
        "{}{}/{}/{}",
        super::CHILD_PREFIX,
        work.slot().owner_key(),
        work.slot().source_site().code(),
        work.id().map_err(crate::CaptureError::Domain)?
    ))
}

fn owner_prefix(work: &AcquisitionWorkV1) -> String {
    format!(
        "{}{}/{}/",
        super::CHILD_PREFIX,
        work.slot().owner_key(),
        work.slot().source_site().code()
    )
}

fn child_work(
    declaration: &PublicResourceDeclarationV1,
    site: census_domain::UsJurisdiction,
    team_id: &str,
    team_url: &str,
) -> StoreResult<AcquisitionWorkV1> {
    let binding = &declaration.binding;
    let owner = binding.owner(site)?;
    let resource = PublicSourceResourceV1::milesplit_team_roster(team_id)
        .map_err(crate::CaptureError::Domain)?;
    let target = format!("{team_url}/roster");
    resource
        .validate_target(site, &target)
        .map_err(crate::CaptureError::Domain)?;
    let root = &owner.directory_request;
    let request = RequestRepresentationV1::normalized_get(
        &target,
        root.representation_headers().to_vec(),
        root.effective_user_agent(),
    )
    .map_err(crate::CaptureError::Domain)?;
    let scope = PublicResourceCaptureScopeV1::new(
        &owner.owner_key,
        site,
        binding.run,
        binding.run_manifest_digest.clone(),
        owner.history,
        binding.plan_digest()?,
        owner.cache_intent,
    )
    .map_err(crate::CaptureError::Domain)?;
    let as_of = ProfileDate::parse(&owner.history.as_of().to_string())
        .map_err(|_| refused("public roster child as of"))?;
    AcquisitionWorkV1::for_public_resource(
        binding.store.clone(),
        &scope,
        site,
        PublicResourceObligationV1::new(resource, as_of),
        request,
    )
    .map_err(|error| crate::CaptureError::Domain(error).into())
}
