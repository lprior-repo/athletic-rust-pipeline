use super::StoreSnapshot;
use crate::StoreResult;
use census_domain::model::{ContractRefV1, RegionMembershipContractV1, StoreIdentity};

impl StoreSnapshot<'_> {
    pub fn region_contract(
        &self,
        reference: &ContractRefV1,
    ) -> StoreResult<RegionMembershipContractV1> {
        crate::region_contracts::read::read(&self.snapshot, self.meta, reference)
    }

    pub fn selected_region_contract(
        &self,
        identity: &StoreIdentity,
        reference: &ContractRefV1,
    ) -> StoreResult<RegionMembershipContractV1> {
        crate::region_contracts::ensure_store(identity, &self.store_identity()?)?;
        self.region_contract(reference)
    }
}
