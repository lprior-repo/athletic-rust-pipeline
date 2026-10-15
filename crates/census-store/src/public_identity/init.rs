use super::{codec, PublicIdentityError, DECLARATION, HEAD, HISTORY, LIFECYCLE};
use crate::{Store, StoreError, StoreResult, Table};
use census_domain::public_identity_history::genesis;
use census_domain::public_identity_qualification::PublicIdentityQualificationV1;
use fjall::PersistMode;
use std::path::Path;

impl Store {
    pub fn create_public_identity_qualification_v1(
        root: impl AsRef<Path>,
    ) -> StoreResult<PublicIdentityQualificationV1> {
        create(root.as_ref())
    }
}

fn create(root: &Path) -> StoreResult<PublicIdentityQualificationV1> {
    crate::acquisition::init::claim_empty_root(root).map_err(init_error)?;
    let (db, entities, journal, meta, receipts) =
        crate::open_keyspaces(root, crate::DEFAULT_CACHE_BYTES)?;
    crate::acquisition::init::require_claimed_root(root).map_err(init_error)?;
    [&entities, &journal, &meta, &receipts]
        .into_iter()
        .try_for_each(|keyspace| {
            if !keyspace
                .is_empty()
                .map_err(|source| StoreError::Read { source })?
            {
                return Err(PublicIdentityError::ExistingRoot.into());
            }
            Ok::<(), StoreError>(())
        })?;
    let mut batch = db.batch();
    crate::format::stage_initialization(&mut batch, &meta, LIFECYCLE);
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
    let declaration = PublicIdentityQualificationV1::new(identity);
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

fn init_error(error: StoreError) -> StoreError {
    match error {
        StoreError::Capture(
            crate::CaptureError::CaptureInitExistingStore
            | crate::CaptureError::CaptureInitRootNotEmpty,
        ) => PublicIdentityError::ExistingRoot.into(),
        other => other,
    }
}
