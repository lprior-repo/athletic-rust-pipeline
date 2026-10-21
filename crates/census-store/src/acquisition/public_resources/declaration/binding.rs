use super::super::text;
use super::*;
use crate::acquisition::CaptureError;

impl PublicResourceDeclarationBindingV1 {
    pub(crate) fn validate(
        &self,
        capture: &CaptureQualification,
        manifest: &RunManifest,
        profile: &QueryProfileV1,
    ) -> StoreResult<()> {
        self.validate_shape()?;
        if capture.protocol() != CaptureProtocolVersion::V2
            || capture.store_identity() != &self.store
            || manifest.store_identity != self.store.as_str()
            || manifest.run != self.run
            || manifest.cohort != GradYear::CO2027
            || codec::object_digest(capture)? != self.capture_declaration_digest
            || codec::object_digest(manifest)? != self.run_manifest_digest
        {
            return Err(refused("public resource declaration lineage"));
        }
        self.selection
            .ensure_store(&self.store)
            .map_err(|_| refused("public resource selected profile store"))?;
        if profile
            .id()
            .map_err(|_| refused("public resource profile digest"))?
            != *self.selection.profile_id()
            || profile.input().graduation_years.as_slice() != [GradYear::CO2027]
        {
            return Err(refused("public resource selected profile binding"));
        }
        self.validate_owners(manifest, profile)
    }

    pub(crate) fn plan_digest(&self) -> StoreResult<AcquisitionDigest> {
        codec::object_digest(&("public-resource-capture-plan-v1", self))
    }

    pub(crate) fn validate_shape(&self) -> StoreResult<()> {
        text(&self.national_owner_key, 256)?;
        if self.version != 1 || self.owners.is_empty() || self.owners.len() > 51 {
            return Err(refused("public resource declaration shape"));
        }
        self.selection
            .ensure_store(&self.store)
            .map_err(|_| refused("public resource selected profile store"))?;
        for (index, owner) in self.owners.iter().enumerate() {
            let prior = self
                .owners
                .get(..index)
                .ok_or_else(|| refused("public resource owner inventory"))?;
            if prior
                .iter()
                .any(|held| held.site >= owner.site || held.owner_key == owner.owner_key)
            {
                return Err(refused("public resource duplicate or unordered owner"));
            }
            owner.validate_requests()?;
        }
        Ok(())
    }

    pub(crate) fn owner(&self, site: UsJurisdiction) -> StoreResult<&PublicResourceOwnerBindingV1> {
        let mut owners = self.owners.iter().filter(|owner| owner.site == site);
        let owner = owners
            .next()
            .ok_or_else(|| refused("public resource owner absent"))?;
        if owners.next().is_some() {
            return Err(refused("public resource duplicate owner"));
        }
        Ok(owner)
    }

    fn validate_owners(&self, manifest: &RunManifest, profile: &QueryProfileV1) -> StoreResult<()> {
        let places = &profile.input().jurisdictions;
        if self.owners.is_empty()
            || self.owners.len() > 51
            || self.owners.len() != manifest.jurisdictions.len()
            || self.owners.len() != places.len()
        {
            return Err(refused("public resource owner scope"));
        }
        for (index, owner) in self.owners.iter().enumerate() {
            let prior = self
                .owners
                .get(..index)
                .ok_or_else(|| refused("public resource owner inventory"))?;
            if prior
                .iter()
                .any(|held| held.site >= owner.site || held.owner_key == owner.owner_key)
                || manifest
                    .jurisdictions
                    .iter()
                    .filter(|place| **place == owner.site)
                    .count()
                    != 1
                || !places.contains(&owner.site)
                || owner.history.as_of().to_string() != profile.input().as_of.as_str()
            {
                return Err(refused("public resource owner scope"));
            }
            owner.validate_requests()?;
        }
        Ok(())
    }
}

impl PublicResourceOwnerBindingV1 {
    pub(crate) fn validate_requests(&self) -> StoreResult<()> {
        text(&self.owner_key, 64)?;
        let origin = format!(
            "https://{}.milesplit.com",
            self.site.code().to_ascii_lowercase()
        );
        if self.directory_request.original_url() != format!("{origin}/teams")
            || self.robots_request.original_url() != format!("{origin}/robots.txt")
            || !self.directory_request.observed_http_required()
            || !self.robots_request.observed_http_required()
        {
            return Err(refused("public resource bound root request"));
        }
        PublicSourceResourceV1::milesplit_team_directory()
            .validate_target(self.site, self.directory_request.original_url())
            .map_err(CaptureError::Domain)?;
        PublicSourceResourceV1::robots(&origin)
            .and_then(|resource| {
                resource.validate_target(self.site, self.robots_request.original_url())
            })
            .map_err(CaptureError::Domain)?;
        Ok(())
    }
}
