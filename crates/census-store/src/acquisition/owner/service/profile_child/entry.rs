use super::{codec, refused, PublicRosterProfileAssessmentV1, OPERATION_PREFIX, PREFIX};
use crate::acquisition::public_resources::{
    PublicResourceDeclarationV1, PublicSchoolIntakePacketV1,
};
use crate::{Receipt, StoreResult};
use census_domain::model::*;
use census_domain::roster_affiliation_authority::SourceRowLocator;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Entry {
    version: u8,
    declaration: AcquisitionDigest,
    pub(super) work: AcquisitionWorkV1,
    pub(super) packet: crate::PublicSchoolIntakeRefV1,
    athlete_id: String,
    profile_url: String,
    row: SourceRowLocator,
    row_digest: AcquisitionDigest,
    kernel_digest: AcquisitionDigest,
}

impl Entry {
    pub(super) fn assessed(
        declaration: &PublicResourceDeclarationV1,
        assessment: &impl PublicRosterProfileAssessmentV1,
    ) -> StoreResult<Self> {
        let evidence = assessment.evidence();
        evidence.require_row(assessment.row(), assessment.row_digest())?;
        let entry = Self {
            version: 1,
            declaration: declaration.digest()?,
            work: work(declaration, assessment)?,
            packet: evidence.packet().reference(),
            athlete_id: assessment.athlete_id().into(),
            profile_url: assessment.profile_url().into(),
            row: assessment.row(),
            row_digest: assessment.row_digest().clone(),
            kernel_digest: assessment.kernel_digest().clone(),
        };
        entry.validate(declaration)?;
        entry.require_parent(evidence.packet())?;
        Ok(entry)
    }

    pub(super) fn validate(&self, declaration: &PublicResourceDeclarationV1) -> StoreResult<()> {
        let target = PublicSourceResourceV1::milesplit_athlete_profile(&self.athlete_id)
            .map_err(crate::CaptureError::Domain)?;
        target
            .validate_target(self.work.slot().source_site(), &self.profile_url)
            .map_err(crate::CaptureError::Domain)?;
        if self.version != 1
            || self.declaration != declaration.digest()?
            || self.work
                != derive_work(
                    declaration,
                    self.work.slot().source_site(),
                    target,
                    &self.profile_url,
                )?
            || self.row.ordinal() >= 20_000
            || self.row.byte_length() == 0
            || self
                .row
                .byte_offset()
                .checked_add(self.row.byte_length())
                .is_none_or(|end| end > 2 * 1024 * 1024)
        {
            return Err(refused("public profile child binding"));
        }
        Ok(())
    }

    pub(super) fn require_parent(&self, packet: &PublicSchoolIntakePacketV1) -> StoreResult<()> {
        let parent = packet.roster().work();
        if packet.reference() != self.packet
            || parent.slot().source_site() != self.work.slot().source_site()
            || parent.slot().owner_key() != self.work.slot().owner_key()
            || parent.run() != self.work.run()
            || parent.run_manifest_digest() != self.work.run_manifest_digest()
            || parent.history() != self.work.history()
            || parent.source_admission() != self.work.source_admission()
            || parent.cache_intent() != self.work.cache_intent()
            || parent.slot().store_identity() != self.work.slot().store_identity()
        {
            return Err(refused("public profile child roster owner differs"));
        }
        let end = self
            .row
            .byte_offset()
            .checked_add(self.row.byte_length())
            .ok_or(crate::StoreError::CounterOverflow)?;
        let raw = packet
            .roster()
            .body()
            .get(self.row.byte_offset()..end)
            .filter(|raw| !raw.is_empty())
            .ok_or_else(|| refused("public profile child retained row bounds"))?;
        if codec::digest(raw)? != self.row_digest {
            return Err(refused("public profile child retained row differs"));
        }
        Ok(())
    }

    pub(super) fn key(&self) -> String {
        format!(
            "{}{}/{}/{}",
            PREFIX,
            self.work.slot().owner_key(),
            self.work.slot().source_site().code(),
            self.athlete_id
        )
    }

    pub(super) fn operation(&self) -> StoreResult<String> {
        Ok(format!(
            "{OPERATION_PREFIX}{}",
            self.work.id().map_err(crate::CaptureError::Domain)?
        ))
    }

    pub(super) fn require_receipt(
        &self,
        declaration: &PublicResourceDeclarationV1,
        receipt: &Receipt,
    ) -> StoreResult<()> {
        self.validate(declaration)?;
        let at = chrono::DateTime::parse_from_rfc3339(&receipt.at)
            .map_err(|_| refused("public profile child receipt time"))?;
        if receipt.operation != self.operation()?
            || receipt.digest != codec::object_digest(self)?.as_str()
            || at < declaration.admitted_at
            || receipt.appended != 0
            || receipt.credited
        {
            return Err(refused("public profile child receipt"));
        }
        Ok(())
    }
}

fn work(
    declaration: &PublicResourceDeclarationV1,
    assessment: &impl PublicRosterProfileAssessmentV1,
) -> StoreResult<AcquisitionWorkV1> {
    let target = PublicSourceResourceV1::milesplit_athlete_profile(assessment.athlete_id())
        .map_err(crate::CaptureError::Domain)?;
    derive_work(
        declaration,
        assessment
            .evidence()
            .packet()
            .roster()
            .work()
            .slot()
            .source_site(),
        target,
        assessment.profile_url(),
    )
}

fn derive_work(
    declaration: &PublicResourceDeclarationV1,
    site: census_domain::UsJurisdiction,
    resource: PublicSourceResourceV1,
    target: &str,
) -> StoreResult<AcquisitionWorkV1> {
    resource
        .validate_target(site, target)
        .map_err(crate::CaptureError::Domain)?;
    let binding = &declaration.binding;
    let owner = binding.owner(site)?;
    let root = &owner.directory_request;
    let request = RequestRepresentationV1::normalized_get(
        target,
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
        .map_err(|_| refused("public profile child as of"))?;
    AcquisitionWorkV1::for_public_resource(
        binding.store.clone(),
        &scope,
        site,
        PublicResourceObligationV1::new(resource, as_of),
        request,
    )
    .map_err(|error| crate::CaptureError::Domain(error).into())
}
