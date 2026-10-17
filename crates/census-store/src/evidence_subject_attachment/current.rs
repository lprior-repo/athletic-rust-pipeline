use super::{ledger::Ledger, EvidenceSubjectAttachmentStoreError as Error};
use crate::{AcceptedPublicIdentitySnapshot, StoreResult, StoreSnapshot};
use census_domain::evidence_subject_attachment::{
    AttachmentReplayGrantV1, AttachmentSubjectRoleV1, RetainedDocumentSubjectReferenceV1,
};
use census_domain::public_identity_qualification::PublicIdentityQualificationVersion;

mod accepted;
mod binding;
mod capability;
pub(super) mod discovery;
pub(super) mod query;
use accepted::AcceptedOwner;
pub use capability::CurrentEvidenceSubjectAttachmentV1;
pub use discovery::{
    ActiveAttachmentSubjectDiscoveryV1, ActiveAttachmentSubjectV1, DiscoveredAttachmentCaptureV1,
    DiscoveredAttachmentLocatorV1,
};

pub struct CurrentEvidenceSubjectAttachmentReferencesV1<'v, 's> {
    view: &'v StoreSnapshot<'s>,
    ledger: Ledger<'s>,
    accepted: AcceptedOwner<'v>,
}

impl<'v, 's> CurrentEvidenceSubjectAttachmentReferencesV1<'v, 's> {
    pub fn snapshot(&self) -> &StoreSnapshot<'s> {
        self.view
    }

    pub fn inventory(&self) -> &census_domain::evidence_subject_attachment::AttachmentInventoryV1 {
        &self.ledger.inventory
    }
    pub fn store_identity(&self) -> &census_domain::model::StoreIdentity {
        self.accepted.view().store_identity()
    }
    pub fn accepted_public_head(&self) -> &census_domain::public_identity_history::RegistryHead {
        self.accepted.view().head()
    }
    pub fn snapshot_sequence(&self) -> u64 {
        self.view.sequence()
    }
    pub fn discover_active_subjects_v1<'a>(
        &'a self,
        role: AttachmentSubjectRoleV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<ActiveAttachmentSubjectDiscoveryV1<'a>> {
        discovery::discover(&self.ledger, self.accepted.view(), role, grant)
    }
    pub fn discover_active_result_subjects_v1<'a>(
        &'a self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<ActiveAttachmentSubjectDiscoveryV1<'a>> {
        self.discover_active_subjects_v1(AttachmentSubjectRoleV1::ResultRow, grant)
    }
    pub fn freeze_closure(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<super::frozen::FrozenEvidenceSubjectAttachmentBundleV1> {
        super::frozen::capture(self.view, &self.ledger, self.accepted.view(), grant)
    }

    pub fn bind_exact_subject<'a>(
        &'a self,
        expected: &RetainedDocumentSubjectReferenceV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Option<CurrentEvidenceSubjectAttachmentV1<'a, 's>>> {
        grant.ensure_available().map_err(Error::Domain)?;
        let result = binding::bind(self, expected, grant);
        if result.is_err() {
            grant.poison();
        }
        result
    }
}

impl<'s> StoreSnapshot<'s> {
    pub fn read_current_evidence_subject_attachment_references_v1<'v>(
        &'v self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<CurrentEvidenceSubjectAttachmentReferencesV1<'v, 's>> {
        grant.ensure_available().map_err(Error::Domain)?;
        let result = read(self, grant);
        if result.is_err() {
            grant.poison();
        }
        result
    }
}

fn read<'v, 's>(
    view: &'v StoreSnapshot<'s>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<CurrentEvidenceSubjectAttachmentReferencesV1<'v, 's>> {
    grant.charge_steps(1).map_err(Error::Domain)?;
    let identity = view.store_identity()?;
    let ledger = Ledger::load(view, &identity, grant)?;
    grant.charge_steps(1).map_err(Error::Domain)?;
    let accepted = view
        .accepted_public_identity_snapshot()?
        .ok_or(Error::TargetMoved)?;
    grant.charge_steps(1).map_err(Error::Domain)?;
    if accepted.store_identity() != &identity
        || accepted.qualification_version()
            != PublicIdentityQualificationVersion::RetainedAuthorityGraphV3
        || accepted.snapshot_sequence() != view.sequence()
    {
        return Err(Error::TargetMoved.into());
    }
    Ok(CurrentEvidenceSubjectAttachmentReferencesV1 {
        view,
        ledger,
        accepted: AcceptedOwner::Owned(Box::new(accepted)),
    })
}

#[cfg(test)]
mod tests;
