use super::Objects;
use crate::roster_source_issuance::RosterSourceIssuanceStoreError as Error;
use crate::StoreResult;
use census_domain::roster_affiliation_authority::MAX_ROSTER_CLOSURE_BYTES;
use std::ops::Deref;

pub(in crate::roster_source_issuance::issuance) struct Builder {
    objects: Objects,
    total_bytes: usize,
}

impl Builder {
    pub(in crate::roster_source_issuance::issuance) fn new() -> Self {
        Self {
            objects: Objects::new(),
            total_bytes: 0,
        }
    }
    pub(super) fn admit_bytes(&self, additional: usize) -> StoreResult<usize> {
        let total = self
            .total_bytes
            .checked_add(additional)
            .ok_or(Error::BudgetExceeded)?;
        if total > MAX_ROSTER_CLOSURE_BYTES {
            return Err(Error::BudgetExceeded.into());
        }
        Ok(total)
    }
    pub(super) fn retain(
        &mut self,
        digest: String,
        owned: Vec<u8>,
        admitted_total: usize,
    ) -> StoreResult<()> {
        if self.objects.contains_key(&digest) || self.admit_bytes(owned.len())? != admitted_total {
            return Err(Error::Corrupt("stale or duplicate object admission").into());
        }
        self.objects.insert(digest, owned);
        self.total_bytes = admitted_total;
        Ok(())
    }
    pub(in crate::roster_source_issuance::issuance) fn into_objects(self) -> Objects {
        self.objects
    }
}

impl Deref for Builder {
    type Target = Objects;
    fn deref(&self) -> &Self::Target {
        &self.objects
    }
}

#[cfg(test)]
mod tests;
