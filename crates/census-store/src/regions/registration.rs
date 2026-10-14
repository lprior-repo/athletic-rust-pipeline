use super::{read, Envelope, RegionRegistration, RegionRegistryError};
use crate::{Store, StoreError};
use census_domain::model::RegionMembershipContractV1;
use fjall::PersistMode;

mod encode;

impl Store {
    pub fn register_region_contract(
        &self,
        contract: &RegionMembershipContractV1,
    ) -> Result<RegionRegistration, RegionRegistryError> {
        let _appends = self.lock_appends();
        let reference = contract.reference()?;
        let record_key = read::key(&reference);
        if let Some(held) = read::retained(&self.db.snapshot(), &self.meta, &reference)? {
            if &held != contract {
                return Err(RegionRegistryError::ImmutableConflict);
            }
            return Ok(RegionRegistration::Repeated(reference));
        }
        let bytes = encode::envelope(&Envelope {
            version: 1,
            reference: &reference,
            payload: contract,
        })?;
        let mut batch = self.db.batch();
        batch.insert(&self.meta, record_key.as_bytes(), bytes.as_slice());
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })?;
        Ok(RegionRegistration::Registered(reference))
    }
}
