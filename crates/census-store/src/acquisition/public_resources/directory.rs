use super::{refused, PublicResourceRootKindV1};
use crate::VerifiedRetainedOccurrence;
use crate::{AcquisitionReadBudget, PublicResourceCaptureOwnerV1, Store, StoreResult};
use census_domain::model::{AcquisitionDigest, AcquisitionOccurrenceRefV1, AcquisitionWorkV1};
use census_domain::roster_affiliation_authority::SourceRowLocator;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicDirectorySemanticContractV1 {
    MileSplitActiveDirectoryRosterV1,
}

pub struct PublicDirectoryEvidenceV1 {
    pub(super) owner: PublicResourceCaptureOwnerV1,
    pub(super) work: AcquisitionWorkV1,
    pub(super) retained: VerifiedRetainedOccurrence,
}

pub trait PublicDirectoryRosterAssessmentV1 {
    fn evidence(&self) -> &PublicDirectoryEvidenceV1;
    fn team_id(&self) -> &str;
    fn team_url(&self) -> &str;
    fn roster_url(&self) -> &str;
    fn parsed_teams(&self) -> usize;
    fn row(&self) -> SourceRowLocator;
    fn name(&self) -> SourceRowLocator;
    fn city(&self) -> SourceRowLocator;
    fn anchor(&self) -> SourceRowLocator;
    fn row_digest(&self) -> &AcquisitionDigest;
    fn kernel_digest(&self) -> &AcquisitionDigest;
    fn semantic_contract(&self) -> PublicDirectorySemanticContractV1;
}

impl PublicResourceCaptureOwnerV1 {
    pub fn directory_evidence(
        &self,
        store: &Store,
        reference: &AcquisitionOccurrenceRefV1,
        budget: &mut AcquisitionReadBudget,
    ) -> StoreResult<PublicDirectoryEvidenceV1> {
        let work = self.for_root(store, PublicResourceRootKindV1::MileSplitTeamDirectory)?;
        let retained = store.read_retained_acquisition_v1(reference, budget)?;
        require_original(&work, &retained)?;
        Ok(PublicDirectoryEvidenceV1 {
            owner: self.clone(),
            work,
            retained,
        })
    }
}

impl PublicDirectoryEvidenceV1 {
    pub fn work(&self) -> &AcquisitionWorkV1 {
        &self.work
    }

    pub fn retained(&self) -> &VerifiedRetainedOccurrence {
        &self.retained
    }

    pub(super) fn validate_locked(
        &self,
        store: &Store,
        budget: &mut AcquisitionReadBudget,
    ) -> StoreResult<()> {
        self.owner.declaration(store, budget)?;
        let retained = store
            .snapshot_locked()
            .read_retained_acquisition_v1(self.retained.reference(), budget)?;
        require_original(&self.work, &retained)?;
        if retained.prepared() != self.retained.prepared()
            || retained.response() != self.retained.response()
            || retained.body_path() != self.retained.body_path()
        {
            return Err(refused("public directory retained evidence changed"));
        }
        Ok(())
    }
}

fn require_original(
    work: &AcquisitionWorkV1,
    retained: &VerifiedRetainedOccurrence,
) -> StoreResult<()> {
    if retained.reference().attempt().work_id()
        != &work.id().map_err(crate::CaptureError::Domain)?
        || retained.reference().attempt().store_identity() != work.slot().store_identity()
        || retained.reference() != retained.body_reference()
        || retained.revalidation().is_some()
        || retained.response().status() != 200
        || !work.request().observed_http_required()
        || retained.response().observed_http().is_none()
    {
        return Err(refused("public directory original owned capture required"));
    }
    Ok(())
}
