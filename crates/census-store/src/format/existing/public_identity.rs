use super::{existing, OpenPurpose};
use crate::{Store, StoreResult};
use census_domain::public_identity_qualification::PublicIdentityQualificationVersion;
use std::path::Path;

impl Store {
    pub fn open_current_qualification_v3(
        root: impl AsRef<Path>,
    ) -> StoreResult<crate::CaptureQualificationStore> {
        let store = existing(
            root.as_ref(),
            OpenPurpose::CurrentQualificationV3,
            super::super::DEFAULT_CACHE_BYTES,
        )?;
        let record = crate::acquisition::qualification::declaration(
            &store.db.snapshot(),
            &store.meta,
            &store.journal,
            &store.store_format()?,
        )?
        .ok_or(crate::CaptureError::CaptureCapabilityMissing)?;
        Ok(crate::CaptureQualificationStore::new(store, record))
    }

    pub fn open_public_identity_qualification_v3(
        root: impl AsRef<Path>,
    ) -> StoreResult<crate::PublicIdentityQualificationStore> {
        open_public_identity(
            root.as_ref(),
            PublicIdentityQualificationVersion::RetainedAuthorityGraphV3,
        )
    }
    pub fn open_public_identity_qualification_v1(
        _root: impl AsRef<Path>,
    ) -> StoreResult<crate::PublicIdentityQualificationStore> {
        Err(crate::PublicIdentityError::QualificationVersionMismatch {
            expected: PublicIdentityQualificationVersion::RetainedAuthorityGraphV3,
            found: PublicIdentityQualificationVersion::SourceBoundV1,
        }
        .into())
    }
    pub fn open_public_identity_qualification_v2(
        _root: impl AsRef<Path>,
    ) -> StoreResult<crate::PublicIdentityQualificationStore> {
        Err(crate::PublicIdentityError::QualificationVersionMismatch {
            expected: PublicIdentityQualificationVersion::RetainedAuthorityGraphV3,
            found: PublicIdentityQualificationVersion::RetainedAuthorityContextV2,
        }
        .into())
    }
    pub fn replay_historical_public_identity(
        root: impl AsRef<Path>,
    ) -> StoreResult<crate::AcceptedPublicIdentitySnapshot> {
        let store = existing(
            root.as_ref(),
            OpenPurpose::PublicIdentityHistory,
            super::super::DEFAULT_CACHE_BYTES,
        )?;
        store
            .snapshot()
            .accepted_public_identity_snapshot()?
            .ok_or_else(|| crate::PublicIdentityError::OptInRequired.into())
    }
}

fn open_public_identity(
    root: &Path,
    version: PublicIdentityQualificationVersion,
) -> StoreResult<crate::PublicIdentityQualificationStore> {
    let store = existing(
        root,
        OpenPurpose::PublicIdentityExecution(version),
        super::super::DEFAULT_CACHE_BYTES,
    )?;
    let ledger = crate::public_identity::validate(
        &store.db.snapshot(),
        &store.entities,
        &store.meta,
        &store.journal,
        &store.receipts,
        &store.store_format()?,
    )?
    .ok_or(crate::PublicIdentityError::OptInRequired)?;
    Ok(crate::PublicIdentityQualificationStore::new(
        store,
        &ledger.declaration,
    ))
}
