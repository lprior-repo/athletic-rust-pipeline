use crate::{StoreError, StoreResult};
use sha2::{Digest, Sha256};

#[cfg(test)]
mod ceiling_tests;
#[cfg(test)]
mod tests;
pub(crate) mod v1;
pub(crate) mod v2;
pub use v1::replay_frozen_public_identity_v1;
pub use v2::{
    replay_frozen_public_identity_v2, CapturedFrozenPublicIdentityV2, FrozenPublicIdentityBundleV2,
};
pub use v2::{FrozenPublicIdentityDemandV2, MAX_FROZEN_PUBLIC_IDENTITY_DEMAND_JSON_BYTES};

pub const MAX_FROZEN_PUBLIC_IDENTITY_BYTES: usize = 4 * 1024 * 1024 * 1024;

fn bound(bytes: usize, max: usize) -> StoreResult<()> {
    if bytes > max {
        return Err(StoreError::RowTooLarge { bytes, max });
    }
    Ok(())
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
