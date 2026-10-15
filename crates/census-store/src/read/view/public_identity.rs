use super::StoreSnapshot;
use crate::{PublicIdentityError, StoreResult};
use census_domain::model::{AthleteCandidateId, StoreIdentity};
use census_domain::public_identity_history::{
    DecodedCandidateHistory, PublicResolution, RegistryHead,
};

pub struct AcceptedPublicIdentitySnapshot {
    identity: StoreIdentity,
    history: DecodedCandidateHistory,
}

impl AcceptedPublicIdentitySnapshot {
    pub fn store_identity(&self) -> &StoreIdentity {
        &self.identity
    }
    pub fn head(&self) -> &RegistryHead {
        self.history.projection().head()
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
        Ok(ledger.map(|ledger| AcceptedPublicIdentitySnapshot {
            identity: ledger.declaration.store_identity().clone(),
            history: ledger.history,
        }))
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
