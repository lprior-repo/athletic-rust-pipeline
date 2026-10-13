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

impl StoreSnapshot<'_> {
    pub fn profile(
        &self,
        id: &census_domain::model::ProfileId,
    ) -> StoreResult<census_domain::model::QueryProfileV1> {
        crate::profiles::read::retained(&self.snapshot, self.meta, id)?
            .ok_or_else(|| crate::ProfileRegistryError::MissingProfile { id: id.clone() }.into())
    }

    pub fn selected_profile(
        &self,
        selection: &census_domain::model::ProfileSelectionV1,
    ) -> StoreResult<census_domain::model::QueryProfileV1> {
        selection
            .ensure_store(&self.store_identity()?)
            .map_err(crate::ProfileRegistryError::Selection)?;
        self.profile(selection.profile_id())
    }
}
