use census_domain::model::{InstallationConfigV1, InstallationEnrollmentV1};
use fjall::PersistMode;

use super::{
    eligibility, read, InstallationEnrollmentOutcome, InstallationError, InstallationState,
    CONFIG_PREFIX, ENROLLMENT_KEY,
};
use crate::{Store, StoreError, StoreResult};

pub struct InstallationControl {
    store: Store,
}

impl InstallationControl {
    pub(crate) fn new(store: Store) -> Self {
        Self { store }
    }

    pub fn show(&self) -> StoreResult<InstallationState> {
        let snapshot = self.store.db.snapshot();
        read::validate_format(
            &snapshot,
            &self.store.meta,
            &self.store.journal,
            self.store.store_format()?.lifecycle_revision,
        )
    }

    pub fn enroll(
        &mut self,
        config: &InstallationConfigV1,
    ) -> StoreResult<InstallationEnrollmentOutcome> {
        if let Some(enrollment) = self.precheck(config)? {
            return Ok(InstallationEnrollmentOutcome::Repeated(enrollment));
        }
        let identity = self.store.store_identity()?;
        let _writes = self.store.lock_appends();
        if let Some(enrollment) = check(&self.store, config)? {
            return Ok(InstallationEnrollmentOutcome::Repeated(enrollment));
        }
        let retained = match crate::lineage::read(&self.store.db.snapshot(), &self.store.journal)? {
            crate::lineage::RetainedIdentity::Present(identity) => identity,
            crate::lineage::RetainedIdentity::Absent => {
                return Err(InstallationError::ForeignStore.into())
            }
        };
        if retained != identity {
            return Err(InstallationError::ForeignStore.into());
        }
        let id = config.id().map_err(InstallationError::Domain)?;
        let enrollment = InstallationEnrollmentV1::new(identity, id.clone());
        let payload = crate::records::codec::encode_canonical(&read::ConfigEnvelope {
            version: 1,
            id: &id,
            payload: config,
        })?;
        let enrolled = crate::records::codec::encode_canonical(&enrollment)?;
        let mut batch = self.store.db.batch();
        batch.insert(&self.store.meta, format!("{CONFIG_PREFIX}{id}"), payload);
        batch.insert(&self.store.meta, ENROLLMENT_KEY, enrolled);
        crate::meta::put_text(
            &mut batch,
            &self.store.meta,
            crate::format::LIFECYCLE_REVISION_KEY,
            "3",
        );
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })?;
        Ok(InstallationEnrollmentOutcome::Enrolled(enrollment))
    }

    fn precheck(
        &self,
        config: &InstallationConfigV1,
    ) -> StoreResult<Option<InstallationEnrollmentV1>> {
        let _writes = self.store.lock_appends();
        check(&self.store, config)
    }
}

fn check(
    store: &Store,
    config: &InstallationConfigV1,
) -> StoreResult<Option<InstallationEnrollmentV1>> {
    let snapshot = store.db.snapshot();
    let state = read::validate_format(
        &snapshot,
        &store.meta,
        &store.journal,
        store.store_format()?.lifecycle_revision,
    )?;
    match state {
        InstallationState::Enrolled {
            enrollment,
            config: held,
        } if &held == config => Ok(Some(enrollment)),
        InstallationState::Enrolled { .. } => Err(InstallationError::ImmutableConflict.into()),
        InstallationState::NotEnrolled => {
            eligibility::require_empty(store)?;
            let profile = crate::profiles::read::retained(
                &snapshot,
                &store.meta,
                config.initial_profile_id(),
            )?
            .ok_or_else(|| crate::ProfileRegistryError::MissingProfile {
                id: config.initial_profile_id().clone(),
            })?;
            let _compiled = census_domain::profile_execution::compile_profile(&profile)
                .map_err(InstallationError::ProfileExecution)?;
            Ok(None)
        }
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
