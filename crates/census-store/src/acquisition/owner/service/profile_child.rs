use crate::acquisition::public_resources::{codec, intake, refused, PublicSchoolIntakePacketV1};
use crate::{AcquisitionReadBudget, PublicResourceCaptureOwnerV1, Store, StoreResult};
use census_domain::model::AcquisitionDigest;
use census_domain::roster_affiliation_authority::SourceRowLocator;

pub(crate) mod capture;
mod counter;
mod entry;
mod read;
pub(crate) mod startup;
mod write;

use entry::Entry;
pub(in crate::acquisition) use read::for_work;

const PREFIX: &str = "public_resource_child_profile/v1/entry/";
const OPERATION_PREFIX: &str = "public_resource_child_profile_v1:";
const COUNT_PREFIX: &str = "public_resource_child_profile/v1/count/";
const MAX_PROFILES_PER_OWNER: u32 = 4_000_000;

pub struct PreparedPublicProfileChildV1 {
    owner: PublicResourceCaptureOwnerV1,
    entry: Entry,
}

impl PreparedPublicProfileChildV1 {
    pub fn work(&self) -> &census_domain::model::AcquisitionWorkV1 {
        &self.entry.work
    }
}

pub struct PublicRosterProfileEvidenceV1<'p> {
    store: &'p Store,
    owner: PublicResourceCaptureOwnerV1,
    packet: &'p PublicSchoolIntakePacketV1,
}

pub(crate) trait PublicRosterProfileAssessmentV1 {
    fn evidence(&self) -> &PublicRosterProfileEvidenceV1<'_>;
    fn athlete_id(&self) -> &str;
    fn profile_url(&self) -> &str;
    fn row(&self) -> SourceRowLocator;
    fn row_digest(&self) -> &AcquisitionDigest;
    fn kernel_digest(&self) -> &AcquisitionDigest;
}

impl PublicResourceCaptureOwnerV1 {
    pub fn roster_profile_evidence<'p>(
        &self,
        store: &'p Store,
        packet: &'p PublicSchoolIntakePacketV1,
    ) -> StoreResult<PublicRosterProfileEvidenceV1<'p>> {
        let _writes = store.lock_appends();
        let evidence = PublicRosterProfileEvidenceV1 {
            store,
            owner: self.clone(),
            packet,
        };
        evidence.validate_locked(store, &mut AcquisitionReadBudget::for_work())?;
        Ok(evidence)
    }
}

impl PublicRosterProfileEvidenceV1<'_> {
    pub fn prepare(
        &self,
        athlete_id: &str,
        profile_url: &str,
        row: SourceRowLocator,
        row_digest: &AcquisitionDigest,
        kernel_digest: &AcquisitionDigest,
    ) -> StoreResult<PreparedPublicProfileChildV1> {
        self.store.prepare_public_profile_child(&Assessment {
            evidence: self,
            athlete_id,
            profile_url,
            row,
            row_digest,
            kernel_digest,
        })
    }

    pub fn packet(&self) -> &PublicSchoolIntakePacketV1 {
        self.packet
    }

    fn validate_locked(
        &self,
        store: &Store,
        budget: &mut AcquisitionReadBudget,
    ) -> StoreResult<()> {
        self.owner.declaration(store, budget)?;
        self.owner.matches_work(self.packet.roster().work())?;
        let snapshot = store.db.snapshot();
        let retained = store
            .snapshot_locked()
            .read_retained_capture_root_v1(
                crate::RetainedCaptureRootSchemaV1::ProviderSchoolObservedPacketV1,
                self.packet.reference().as_str(),
                &mut census_domain::evidence_subject_attachment::AttachmentReplayGrantV1::new(),
            )?
            .ok_or_else(|| refused("public profile child retained roster absent"))?;
        if retained.payload().get().as_bytes() != self.packet.encode()? {
            return Err(refused("public profile child retained roster differs"));
        }
        intake::read_admission(
            &snapshot,
            &store.meta,
            &store.journal,
            &store.receipts,
            self.packet,
            budget,
        )?;
        Ok(())
    }

    fn require_row(&self, row: SourceRowLocator, digest: &AcquisitionDigest) -> StoreResult<()> {
        let end = row
            .byte_offset()
            .checked_add(row.byte_length())
            .ok_or(crate::StoreError::CounterOverflow)?;
        let raw = self
            .packet
            .roster()
            .body()
            .get(row.byte_offset()..end)
            .filter(|bytes| !bytes.is_empty())
            .ok_or_else(|| refused("public profile child roster row bounds"))?;
        if row.ordinal() >= 20_000 || codec::digest(raw)? != *digest {
            return Err(refused("public profile child roster row digest"));
        }
        Ok(())
    }
}

struct Assessment<'e, 'p> {
    evidence: &'e PublicRosterProfileEvidenceV1<'p>,
    athlete_id: &'e str,
    profile_url: &'e str,
    row: SourceRowLocator,
    row_digest: &'e AcquisitionDigest,
    kernel_digest: &'e AcquisitionDigest,
}

impl PublicRosterProfileAssessmentV1 for Assessment<'_, '_> {
    fn evidence(&self) -> &PublicRosterProfileEvidenceV1<'_> {
        self.evidence
    }
    fn athlete_id(&self) -> &str {
        self.athlete_id
    }
    fn profile_url(&self) -> &str {
        self.profile_url
    }
    fn row(&self) -> SourceRowLocator {
        self.row
    }
    fn row_digest(&self) -> &AcquisitionDigest {
        self.row_digest
    }
    fn kernel_digest(&self) -> &AcquisitionDigest {
        self.kernel_digest
    }
}
