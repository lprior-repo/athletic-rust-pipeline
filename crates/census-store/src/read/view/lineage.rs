use census_domain::model::StoreIdentity;

use crate::lineage::{self, RetainedIdentity};
use crate::{Store, StoreLineageError, StoreResult};

use super::StoreSnapshot;

impl StoreSnapshot<'_> {
    pub fn store_identity(&self) -> StoreResult<StoreIdentity> {
        match lineage::read(&self.snapshot, self.journal)? {
            RetainedIdentity::Present(identity) => Ok(identity),
            RetainedIdentity::Absent => Err(StoreLineageError::Missing.into()),
        }
    }

    pub fn ensure_origin(&self, store: &Store) -> StoreResult<()> {
        self.revision.ensure_origin(store, "read export snapshot")
    }
}
