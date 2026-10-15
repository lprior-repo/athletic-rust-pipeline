use super::{codec, PublicIdentityError, DECLARATION, HEAD, HISTORY};
use crate::{Store, StoreError, StoreResult, Table};
use census_domain::public_identity_history::genesis;
#[cfg(any(test, feature = "native-fault-injection"))]
use census_domain::public_identity_qualification::PublicIdentityQualificationV1;
use census_domain::public_identity_qualification::{
    PublicIdentityQualification, PublicIdentityQualificationV2, PublicIdentityQualificationVersion,
};
use fjall::PersistMode;
use std::path::Path;

impl Store {
    pub fn create_public_identity_qualification_v2(
        root: impl AsRef<Path>,
    ) -> StoreResult<PublicIdentityQualificationV2> {
        match create(
            root.as_ref(),
            PublicIdentityQualificationVersion::RetainedAuthorityContextV2,
        )? {
            PublicIdentityQualification::V2(declaration) => Ok(declaration),
            PublicIdentityQualification::V1(_) => {
                Err(PublicIdentityError::Corrupt("new declaration version").into())
            }
        }
    }

    #[cfg(any(test, feature = "native-fault-injection"))]
    pub fn create_public_identity_qualification_v1(
        root: impl AsRef<Path>,
    ) -> StoreResult<PublicIdentityQualificationV1> {
        match create(
            root.as_ref(),
            PublicIdentityQualificationVersion::SourceBoundV1,
        )? {
            PublicIdentityQualification::V1(declaration) => Ok(declaration),
            PublicIdentityQualification::V2(_) => {
                Err(PublicIdentityError::Corrupt("new declaration version").into())
            }
        }
    }
}

fn create(
    root: &Path,
    version: PublicIdentityQualificationVersion,
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
    let declaration = match version {
        PublicIdentityQualificationVersion::SourceBoundV1 => PublicIdentityQualification::V1(
            census_domain::public_identity_qualification::PublicIdentityQualificationV1::new(
                identity,
            ),
        ),
        PublicIdentityQualificationVersion::RetainedAuthorityContextV2 => {
            PublicIdentityQualification::V2(PublicIdentityQualificationV2::new(identity))
        }
    };
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
mod tests {
    use super::*;
    use fjall::Readable;

    type TestResult = Result<(), Box<dyn std::error::Error>>;
    type PhysicalState = Vec<Vec<(Vec<u8>, Vec<u8>)>>;

    fn retained(store: &Store) -> Result<PhysicalState, Box<dyn std::error::Error>> {
        let snapshot = store.db.snapshot();
        [
            &store.entities,
            &store.journal,
            &store.meta,
            &store.receipts,
        ]
        .into_iter()
        .map(|keyspace| {
            snapshot
                .iter(keyspace)
                .take(128)
                .map(|guard| {
                    let (key, value) = guard.into_inner()?;
                    Ok((key.to_vec(), value.to_vec()))
                })
                .collect()
        })
        .collect()
    }

    #[test]
    fn current_qualification_creates_lifecycle_six_and_reopens_without_rewriting() -> TestResult {
        let directory = tempfile::tempdir()?;
        let root = directory.path().join("current");
        let declaration = Store::create_public_identity_qualification_v2(&root)?;
        let (store, writer) = Store::open_public_identity_qualification_v2(&root)?.into_parts();
        check!(eq; store.store_format()?.lifecycle_revision, Some(6));
        check!(eq; writer.store_identity(), declaration.store_identity());
        check!(eq; writer.qualification_version(), PublicIdentityQualificationVersion::RetainedAuthorityContextV2);
        writer.require_current_qualification()?;
        check!(eq; writer.head(&store)?, genesis(declaration.store_identity())?);
        let before = retained(&store)?;
        drop(store);
        check!(matches!(
            Store::open_public_identity_qualification_v1(&root),
            Err(StoreError::PublicIdentity(
                PublicIdentityError::QualificationVersionMismatch {
                    expected: PublicIdentityQualificationVersion::SourceBoundV1,
                    found: PublicIdentityQualificationVersion::RetainedAuthorityContextV2,
                }
            ))
        ));
        check!(Store::open_existing(&root).is_err());
        let (store, writer) = Store::open_public_identity_qualification_v2(&root)?.into_parts();
        check!(eq; retained(&store)?, before);
        check!(eq; writer.head(&store)?, genesis(declaration.store_identity())?);
        Ok(())
    }

    #[test]
    fn historical_qualification_is_not_retained_context_certification() -> TestResult {
        let directory = tempfile::tempdir()?;
        let root = directory.path().join("historical");
        let declaration = Store::create_public_identity_qualification_v1(&root)?;
        let (store, writer) = Store::open_public_identity_qualification_v1(&root)?.into_parts();
        check!(eq; store.store_format()?.lifecycle_revision, Some(5));
        check!(eq; writer.qualification_version(), PublicIdentityQualificationVersion::SourceBoundV1);
        check!(writer.require_current_qualification().is_err());
        check!(eq; writer.head(&store)?, genesis(declaration.store_identity())?);
        let before = retained(&store)?;
        drop(store);
        check!(matches!(
            Store::open_public_identity_qualification_v2(&root),
            Err(StoreError::PublicIdentity(
                PublicIdentityError::QualificationVersionMismatch {
                    expected: PublicIdentityQualificationVersion::RetainedAuthorityContextV2,
                    found: PublicIdentityQualificationVersion::SourceBoundV1,
                }
            ))
        ));
        check!(Store::create_public_identity_qualification_v2(&root).is_err());
        let (store, _) = Store::open_public_identity_qualification_v1(&root)?.into_parts();
        check!(eq; retained(&store)?, before);
        Ok(())
    }

    #[test]
    fn current_open_does_not_create_and_creation_does_not_adopt_artifacts() -> TestResult {
        let directory = tempfile::tempdir()?;
        let root = directory.path().join("absent");
        check!(Store::open_public_identity_qualification_v2(&root).is_err());
        check!(!root.exists());
        std::fs::create_dir(&root)?;
        let artifact = root.join("historical.body");
        std::fs::write(&artifact, b"retained evidence")?;
        check!(matches!(
            Store::create_public_identity_qualification_v2(&root),
            Err(StoreError::PublicIdentity(
                PublicIdentityError::ExistingRoot
            ))
        ));
        check!(eq; std::fs::read(&artifact)?, b"retained evidence");
        check!(eq; std::fs::read_dir(&root)?.count(), 1);
        Ok(())
    }
}
