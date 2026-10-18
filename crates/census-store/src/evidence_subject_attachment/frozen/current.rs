use super::super::{
    current::{accepted::AcceptedOwner, discovery, query},
    ledger::{self, VerifiedLedger},
};
use super::*;
use crate::read::JournalNamespaceContentsInventory;
use crate::AcceptedPublicIdentitySnapshot;
use census_domain::evidence_subject_attachment::{
    AttachmentInventoryV1, AttachmentReplayContinuationV1, AttachmentSubjectRoleV1,
    RetainedDocumentSubjectReferenceV1,
};
use census_domain::model::StoreIdentity;
use census_domain::public_identity_history::RegistryHead;
use census_domain::public_identity_qualification::PublicIdentityQualificationVersion;

pub struct FrozenCurrentEvidenceSubjectAttachmentReferencesV1<'b> {
    pub(super) source: FrozenRetainedCaptureSourceV1<'b>,
    pub(super) ledger: VerifiedLedger<JournalNamespaceContentsInventory>,
    accepted: AcceptedOwner<'b>,
    pub(super) continuation: AttachmentReplayContinuationV1,
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
    checked(source, AcceptedOwner::Borrowed(accepted), grant)
}

fn checked<'b>(
    source: FrozenRetainedCaptureSourceV1<'b>,
    accepted: AcceptedOwner<'b>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<FrozenCurrentEvidenceSubjectAttachmentReferencesV1<'b>> {
    steps(grant, 1)?;
    let header = &source.archive;
    let snapshot = accepted.view();
    if header.store != snapshot.store_identity().as_str()
        || header.snapshot_sequence != snapshot.snapshot_sequence()
        || header.public_head.sequence != snapshot.head().sequence
        || header.public_head.digest != snapshot.head().digest
        || snapshot.qualification_version()
            != PublicIdentityQualificationVersion::RetainedAuthorityGraphV3
    {
        return Err(Error::TargetMoved.into());
    }
    let ledger = ledger::read_retained(&source, snapshot.store_identity(), grant)?;
    steps(grant, 1)?;
    if header.inventory.version != ledger.inventory.version
        || header.inventory.records != ledger.inventory.records
        || header.inventory.digest != ledger.inventory.digest.as_str()
    {
        return Err(Error::Corrupt("frozen attachment inventory differs").into());
    }
    roots::require(&source, &ledger, grant)?;
    let continuation = grant.checkpoint().map_err(Error::Domain)?;
    Ok(FrozenCurrentEvidenceSubjectAttachmentReferencesV1 {
        source,
        ledger,
        accepted,
        continuation,
    })
}

impl<'b> FrozenCurrentEvidenceSubjectAttachmentReferencesV1<'b> {
    pub(crate) fn require_original_grant(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<()> {
        self.continuation.require(grant).map_err(Error::Domain)?;
        Ok(())
    }

    pub(crate) fn accepted_snapshot(&self) -> &AcceptedPublicIdentitySnapshot {
        self.accepted.view()
    }

    pub(crate) fn contains_retained_capture_root(
        &self,
        root: &super::super::RetainedCaptureRootV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<bool> {
        self.require_original_grant(grant)?;
        self.source.contains_retained_capture_root(root, grant)
    }
    pub fn source(&self) -> &FrozenRetainedCaptureSourceV1<'b> {
        &self.source
    }
    pub fn inventory(&self) -> &AttachmentInventoryV1 {
        &self.ledger.inventory
    }
    pub fn store_identity(&self) -> &StoreIdentity {
        self.accepted.view().store_identity()
    }
    pub fn accepted_public_head(&self) -> &RegistryHead {
        self.accepted.view().head()
    }
    pub fn snapshot_sequence(&self) -> u64 {
        self.source.archive.snapshot_sequence
    }
    pub(crate) fn retained_records(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<&[census_domain::evidence_subject_attachment::ReviewedReferenceRecordV1]> {
        self.continuation.require(grant).map_err(Error::Domain)?;
        Ok(&self.ledger.records)
    }
    pub fn discover_active_subjects_v1<'a>(
        &'a self,
        role: AttachmentSubjectRoleV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<discovery::ActiveAttachmentSubjectDiscoveryV1<'a>> {
        self.require_original_grant(grant)?;
        discovery::discover(&self.ledger, self.accepted.view(), role, grant)
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
        self.require_original_grant(grant)?;
        let result =
            query::bind(&self.ledger, self.accepted.view(), expected, grant).map(|parts| {
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

impl<'b> FrozenCurrentEvidenceSubjectAttachmentReferencesV1<'b> {
    pub(crate) fn from_borrowed_bytes(
        bytes: &'b [u8],
        reference: &AttachmentObjectReferenceV1,
        accepted: AcceptedPublicIdentitySnapshot,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Self> {
        grant.ensure_available().map_err(Error::Domain)?;
        let result = (|| {
            let source = FrozenRetainedCaptureSourceV1::read_bytes(bytes, reference, grant)?;
            grant
                .charge_bytes(std::mem::size_of::<AcceptedPublicIdentitySnapshot>())
                .map_err(Error::Domain)?;
            checked(source, AcceptedOwner::Owned(Box::new(accepted)), grant)
        })();
        if result.is_err() {
            grant.poison();
        }
        result
    }
}
