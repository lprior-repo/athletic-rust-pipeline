use super::{codec, PublicIdentityError, DECLARATION, HEAD, HISTORY};
use crate::{Store, StoreError, StoreResult, Table};
use census_domain::public_identity_history::genesis;
#[cfg(any(test, feature = "native-fault-injection"))]
use census_domain::public_identity_qualification::PublicIdentityQualificationV1;
use census_domain::public_identity_qualification::{
    PublicIdentityQualification, PublicIdentityQualificationV2, PublicIdentityQualificationV3,
    PublicIdentityQualificationVersion,
};
use fjall::PersistMode;
use std::path::Path;

impl Store {
    pub fn create_current_qualification_v3(
        root: impl AsRef<Path>,
    ) -> StoreResult<PublicIdentityQualificationV3> {
        match create(
            root.as_ref(),
            PublicIdentityQualificationVersion::RetainedAuthorityGraphV3,
            Some(census_domain::model::CaptureProtocolVersion::V2),
        )? {
            PublicIdentityQualification::V3(declaration) => Ok(declaration),
            _ => Err(PublicIdentityError::Corrupt("new graph declaration version").into()),
        }
    }

    pub fn create_public_identity_qualification_v3(
        root: impl AsRef<Path>,
    ) -> StoreResult<PublicIdentityQualificationV3> {
        match create(
            root.as_ref(),
            PublicIdentityQualificationVersion::RetainedAuthorityGraphV3,
            None,
        )? {
            PublicIdentityQualification::V3(declaration) => Ok(declaration),
            _ => Err(PublicIdentityError::Corrupt("new graph declaration version").into()),
        }
    }

    pub fn create_public_identity_qualification_v2(
        _root: impl AsRef<Path>,
    ) -> StoreResult<PublicIdentityQualificationV2> {
        Err(PublicIdentityError::QualificationVersionMismatch {
            expected: PublicIdentityQualificationVersion::RetainedAuthorityGraphV3,
            found: PublicIdentityQualificationVersion::RetainedAuthorityContextV2,
        }
        .into())
    }

    #[cfg(any(test, feature = "native-fault-injection"))]
    pub fn create_public_identity_qualification_v1(
        _root: impl AsRef<Path>,
    ) -> StoreResult<PublicIdentityQualificationV1> {
        Err(PublicIdentityError::QualificationVersionMismatch {
            expected: PublicIdentityQualificationVersion::RetainedAuthorityGraphV3,
            found: PublicIdentityQualificationVersion::SourceBoundV1,
        }
        .into())
    }
}

#[cfg(test)]
pub(crate) fn create_historical_test_fixture(
    root: &Path,
    version: PublicIdentityQualificationVersion,
) -> StoreResult<PublicIdentityQualification> {
    match version {
        PublicIdentityQualificationVersion::SourceBoundV1
        | PublicIdentityQualificationVersion::RetainedAuthorityContextV2 => {
            create(root, version, None)
        }
        PublicIdentityQualificationVersion::RetainedAuthorityGraphV3 => {
            Err(PublicIdentityError::Corrupt("historical test fixture version").into())
        }
    }
}

fn create(
    root: &Path,
    version: PublicIdentityQualificationVersion,
    capture: Option<census_domain::model::CaptureProtocolVersion>,
) -> StoreResult<PublicIdentityQualification> {
    crate::acquisition::init::claim_empty_root(root).map_err(init_error)?;
    let (db, entities, journal, meta, receipts) =
        crate::open_keyspaces(root, crate::DEFAULT_CACHE_BYTES)?;
    crate::acquisition::init::require_claimed_root(root).map_err(init_error)?;
    require_empty_keyspaces([&entities, &journal, &meta, &receipts])?;
    let mut batch = db.batch();
    crate::format::stage_initialization(&mut batch, &meta, version.lifecycle_revision());
    Table::ALL.into_iter().for_each(|table| {
        crate::meta::put_text(
            &mut batch,
            &meta,
            &format!("sequence:{}", table.file()),
            "0",
        );
        crate::meta::put_text(&mut batch, &meta, &format!("rows:{}", table.file()), "0");
    });
    let identity = crate::lineage::stage_new(&mut batch, &journal, root)?;
    let head = genesis(&identity).map_err(PublicIdentityError::from)?;
    if let Some(protocol) = capture {
        crate::acquisition::init::stage_qualification(
            &mut batch,
            &meta,
            identity.clone(),
            protocol,
        )?;
    }
    let declaration = declaration(version, identity)?;
    batch.insert(&meta, DECLARATION, codec::encode(&declaration, 65_536)?);
    batch.insert(&meta, HEAD, codec::encode(&head, 1024)?);
    batch.insert(&meta, HISTORY, b"[]");
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })?;
    crate::ensure_dirs(root)?;
    crate::fs::fsync_dir(root)?;
    let ledger = super::validate(
        &db.snapshot(),
        &entities,
        &meta,
        &journal,
        &receipts,
        &crate::format::read_format(&meta)?,
    )?
    .ok_or(PublicIdentityError::Corrupt("new declaration absent"))?;
    if ledger.declaration != declaration {
        return Err(PublicIdentityError::Corrupt("new declaration mismatch").into());
    }
    Ok(declaration)
}

fn declaration(
    version: PublicIdentityQualificationVersion,
    identity: census_domain::model::StoreIdentity,
) -> StoreResult<PublicIdentityQualification> {
    match version {
        PublicIdentityQualificationVersion::SourceBoundV1 => {
            Err(PublicIdentityError::Corrupt("removed public identity V1 declaration").into())
        }
        PublicIdentityQualificationVersion::RetainedAuthorityContextV2 => Ok(
            PublicIdentityQualification::V2(PublicIdentityQualificationV2::new(identity)),
        ),
        PublicIdentityQualificationVersion::RetainedAuthorityGraphV3 => Ok(
            PublicIdentityQualification::V3(PublicIdentityQualificationV3::new(identity)),
        ),
    }
}

fn require_empty_keyspaces(keyspaces: [&fjall::Keyspace; 4]) -> StoreResult<()> {
    keyspaces.into_iter().try_for_each(|keyspace| {
        if !keyspace
            .is_empty()
            .map_err(|source| StoreError::Read { source })?
        {
            return Err(PublicIdentityError::ExistingRoot.into());
        }
        Ok(())
    })
}

fn init_error(error: StoreError) -> StoreError {
    match error {
        StoreError::Capture(
            crate::CaptureError::CaptureInitExistingStore
            | crate::CaptureError::CaptureInitRootNotEmpty,
        ) => PublicIdentityError::ExistingRoot.into(),
        other => other,
    }
}

#[cfg(test)]
mod tests;
