use census_domain::model::{ContractRefV1, RegionMembershipContractV1};

use crate::read::StoreSnapshot;

impl StoreSnapshot<'_> {
    pub fn region_contract(
        &self,
        reference: &ContractRefV1,
    ) -> Result<RegionMembershipContractV1, crate::RegionRegistryError> {
        crate::regions::read::retained(&self.snapshot, self.meta, reference)?.ok_or_else(|| {
            crate::RegionRegistryError::Missing {
                reference: reference.clone(),
            }
        })
    }
}
