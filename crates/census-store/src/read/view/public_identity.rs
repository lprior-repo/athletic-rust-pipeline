use super::StoreSnapshot;
mod component_claims;
mod projection_check;
use crate::{PublicIdentityError, StoreResult};
use census_domain::model::{AthleteCandidateId, StoreIdentity};
use census_domain::public_identity_history::{
    BindingResolution, DecodedCandidateHistory, HistoryRecord, PublicAthleteId, PublicResolution,
    RegistryHead,
};
use census_domain::public_identity_qualification::PublicIdentityQualificationVersion;

pub struct AcceptedPublicIdentitySnapshot {
    identity: StoreIdentity,
    snapshot_sequence: u64,
    qualification: PublicIdentityQualificationVersion,
    history: DecodedCandidateHistory,
}

impl AcceptedPublicIdentitySnapshot {
    pub(crate) fn from_qualified_ledger(
        ledger: crate::public_identity::qualification::Ledger,
        snapshot_sequence: u64,
    ) -> Self {
        Self {
            identity: ledger.declaration.store_identity().clone(),
            qualification: ledger.declaration.version(),
            snapshot_sequence,
            history: ledger.history,
        }
    }
    pub fn qualification_version(&self) -> PublicIdentityQualificationVersion {
        self.qualification
    }
    pub fn store_identity(&self) -> &StoreIdentity {
        &self.identity
    }
    pub fn snapshot_sequence(&self) -> u64 {
        self.snapshot_sequence
    }
    pub fn history_records(&self) -> &[HistoryRecord] {
        self.history.records()
    }
    pub fn head(&self) -> &RegistryHead {
        self.history.projection().head()
    }
    pub fn public_ids(&self) -> impl Iterator<Item = &PublicAthleteId> {
        self.history.projection().public_ids()
    }
    pub fn resolve_public(&self, id: &PublicAthleteId) -> StoreResult<PublicResolution> {
        self.history
            .projection()
            .resolve_public(id)
            .map_err(|error| PublicIdentityError::from(error).into())
    }
    pub fn resolve_binding(&self, binding_id: &str) -> StoreResult<BindingResolution> {
        self.history
            .projection()
            .resolve_binding(binding_id)
            .map_err(|error| PublicIdentityError::from(error).into())
    }
    pub fn correction_history(&self, id: &PublicAthleteId) -> Option<&[u64]> {
        self.history.projection().correction_history(id)
    }
    pub fn affected_record_keys(&self, sequence: u64) -> Option<&[String]> {
        self.history.projection().affected_record_keys(sequence)
    }
    pub fn resolve_candidate(
        &self,
        candidate: &AthleteCandidateId,
    ) -> StoreResult<PublicResolution> {
        self.history
            .projection()
            .resolve_candidate(candidate)
            .map_err(|error| PublicIdentityError::from(error).into())
    }
}

impl StoreSnapshot<'_> {
    pub(crate) fn attachment_public_head(&self) -> StoreResult<RegistryHead> {
        crate::public_identity::attachment_public_head(&self.snapshot, self.meta)
    }
    pub fn accepted_public_identity_snapshot(
        &self,
    ) -> StoreResult<Option<AcceptedPublicIdentitySnapshot>> {
        let format = crate::format::read_capture_format(
            &self.snapshot,
            self.meta,
            &mut crate::AcquisitionReadBudget::for_work(),
        )?;
        let ledger = crate::public_identity::validate(
            &self.snapshot,
            self.entities,
            self.meta,
            self.journal,
            self.receipts,
            &format,
        )?;
        Ok(ledger.map(|ledger| {
            AcceptedPublicIdentitySnapshot::from_qualified_ledger(ledger, self.sequence())
        }))
    }
    pub fn freeze_public_identity_v2(&self) -> StoreResult<crate::FrozenPublicIdentityBundleV2> {
        let accepted = self
            .accepted_public_identity_snapshot()?
            .ok_or(PublicIdentityError::OptInRequired)?;
        crate::public_identity::frozen::v2::capture(
            &self.snapshot,
            self.entities,
            self.meta,
            self.journal,
            self.receipts,
            &accepted,
        )
    }
    pub fn require_legacy_identity_publication(&self) -> StoreResult<()> {
        let format = crate::format::read_capture_format(
            &self.snapshot,
            self.meta,
            &mut crate::AcquisitionReadBudget::for_work(),
        )?;
        crate::public_identity::require_legacy(
            &self.snapshot,
            self.meta,
            self.journal,
            self.receipts,
            &format,
        )
    }
}
