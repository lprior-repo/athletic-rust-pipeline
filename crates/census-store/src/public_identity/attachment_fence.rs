use census_domain::public_identity_history::RegistryHead;
use fjall::{Keyspace, Snapshot};

use super::{codec, PublicIdentityError, HEAD};
use crate::StoreResult;

pub(crate) fn read_head(snapshot: &Snapshot, meta: &Keyspace) -> StoreResult<RegistryHead> {
    let raw = codec::load(snapshot, meta, HEAD, 1024)?
        .ok_or(PublicIdentityError::Corrupt("missing head"))?;
    codec::head(&raw)
}

#[cfg(test)]
mod tests;
