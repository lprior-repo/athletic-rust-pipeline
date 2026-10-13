use super::{key, read, Envelope, ProfileRegistration, ProfileRegistryError};
use crate::{Store, StoreError, StoreResult};
use census_domain::model::QueryProfileV1;
use fjall::PersistMode;

impl Store {
    pub fn register_profile(&self, profile: &QueryProfileV1) -> StoreResult<ProfileRegistration> {
        let _appends = self.lock_appends();
        let id = profile.id().map_err(ProfileRegistryError::Canonical)?;
        let record_key = key(&id);
        if let Some(retained) = self
            .meta
            .get(record_key.as_bytes())
            .map_err(|source| StoreError::Read { source })?
        {
            let held = read::decode(&id, retained.as_ref())?;
            if &held != profile {
                return Err(ProfileRegistryError::ImmutableConflict { id }.into());
            }
            return Ok(ProfileRegistration::Repeated(id));
        }
        let bytes = crate::records::codec::encode_canonical(&Envelope {
            version: 1,
            id: &id,
            payload: profile,
        })?;
        let mut batch = self.db.batch();
        batch.insert(&self.meta, record_key.as_bytes(), bytes);
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })?;
        Ok(ProfileRegistration::Registered(id))
    }
}
