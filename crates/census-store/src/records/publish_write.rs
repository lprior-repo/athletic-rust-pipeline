use super::{active_key, record_key, Digest, Intent, State};
use crate::records::codec::encode;
use crate::{Store, StoreError, StoreResult};

pub(super) fn write(store: &Store, intent: &Intent, active: Option<&Digest>) -> StoreResult<()> {
    let mut batch = store.db.batch();
    stage(store, intent, active, &mut batch)?;
    batch
        .durability(Some(fjall::PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })
}
pub(super) fn stage(
    store: &Store,
    intent: &Intent,
    active: Option<&Digest>,
    batch: &mut fjall::OwnedWriteBatch,
) -> StoreResult<()> {
    let bytes = encode(intent)?;
    batch.insert(
        &store.meta,
        record_key(&intent.request.identity).as_bytes(),
        bytes.as_slice(),
    );
    if let Some(namespace) = active {
        batch.insert(
            &store.meta,
            active_key(namespace).as_bytes(),
            bytes.as_slice(),
        );
    }
    if let State::Published { receipt } = &intent.state {
        batch.insert(
            &store.meta,
            format!(
                "publication/v1/receipt/{}",
                intent.request.identity.as_str()
            )
            .as_bytes(),
            receipt.as_str().as_bytes(),
        );
    }
    Ok(())
}
