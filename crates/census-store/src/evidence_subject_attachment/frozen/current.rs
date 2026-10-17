use super::super::{
    current::{discovery, query},
    ledger::{self, VerifiedLedger},
};
use super::*;
use crate::read::JournalNamespaceContentsInventory;
use crate::AcceptedPublicIdentitySnapshot;
use census_domain::evidence_subject_attachment::{
    AttachmentInventoryV1, AttachmentSubjectRoleV1, RetainedDocumentSubjectReferenceV1,
};
use census_domain::model::StoreIdentity;
use census_domain::public_identity_history::RegistryHead;
use census_domain::public_identity_qualification::PublicIdentityQualificationVersion;

pub struct FrozenCurrentEvidenceSubjectAttachmentReferencesV1<'b> {
    pub(super) source: FrozenRetainedCaptureSourceV1<'b>,
    pub(super) ledger: VerifiedLedger<JournalNamespaceContentsInventory>,
    pub(super) accepted: &'b AcceptedPublicIdentitySnapshot,
}

pub fn replay_frozen_evidence_subject_attachment_references_v1<'b>(
    bundle: &'b FrozenEvidenceSubjectAttachmentBundleV1,
    accepted: &'b AcceptedPublicIdentitySnapshot,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<FrozenCurrentEvidenceSubjectAttachmentReferencesV1<'b>> {
    grant.ensure_available().map_err(Error::Domain)?;
    let result = read(bundle, accepted, grant);
    if result.is_err() {
        grant.poison();
    }
    result
}

fn read<'b>(
    bundle: &'b FrozenEvidenceSubjectAttachmentBundleV1,
    accepted: &'b AcceptedPublicIdentitySnapshot,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<FrozenCurrentEvidenceSubjectAttachmentReferencesV1<'b>> {
    let source = FrozenRetainedCaptureSourceV1::read(bundle, grant)?;
    steps(grant, 1)?;
    let header = &source.archive;
    if header.store != accepted.store_identity().as_str()
        || header.snapshot_sequence != accepted.snapshot_sequence()
        || header.public_head.sequence != accepted.head().sequence
        || header.public_head.digest != accepted.head().digest
        || accepted.qualification_version()
            != PublicIdentityQualificationVersion::RetainedAuthorityGraphV3
    {
        return Err(Error::TargetMoved.into());
    }
    let ledger = ledger::read_retained(&source, accepted.store_identity(), grant)?;
    steps(grant, 1)?;
    if header.inventory.version != ledger.inventory.version
        || header.inventory.records != ledger.inventory.records
        || header.inventory.digest != ledger.inventory.digest.as_str()
    {
        return Err(Error::Corrupt("frozen attachment inventory differs").into());
    }
    roots::require(&source, &ledger, grant)?;
    Ok(FrozenCurrentEvidenceSubjectAttachmentReferencesV1 {
        source,
        ledger,
        accepted,
    })
}

impl<'b> FrozenCurrentEvidenceSubjectAttachmentReferencesV1<'b> {
    pub fn source(&self) -> &FrozenRetainedCaptureSourceV1<'b> {
        &self.source
    }
    pub fn inventory(&self) -> &AttachmentInventoryV1 {
        &self.ledger.inventory
    }
    pub fn store_identity(&self) -> &StoreIdentity {
        self.accepted.store_identity()
    }
    pub fn accepted_public_head(&self) -> &RegistryHead {
        self.accepted.head()
    }
    pub fn snapshot_sequence(&self) -> u64 {
        self.source.archive.snapshot_sequence
    }
    pub fn discover_active_subjects_v1<'a>(
        &'a self,
        role: AttachmentSubjectRoleV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<discovery::ActiveAttachmentSubjectDiscoveryV1<'a>> {
        discovery::discover(&self.ledger, self.accepted, role, grant)
    }
    pub fn discover_active_result_subjects_v1<'a>(
        &'a self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<discovery::ActiveAttachmentSubjectDiscoveryV1<'a>> {
        self.discover_active_subjects_v1(AttachmentSubjectRoleV1::ResultRow, grant)
    }
    pub fn bind_exact_subject<'a>(
        &'a self,
        expected: &RetainedDocumentSubjectReferenceV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Option<FrozenCurrentEvidenceSubjectAttachmentV1<'a, 'b>>> {
        grant.ensure_available().map_err(Error::Domain)?;
        let result = query::bind(&self.ledger, self.accepted, expected, grant).map(|parts| {
            parts.map(|parts| FrozenCurrentEvidenceSubjectAttachmentV1 {
                owner: self,
                record: parts.record,
                subject: parts.subject,
                receipt: parts.receipt,
            })
        });
        if result.is_err() {
            grant.poison();
        }
        result
    }
}
