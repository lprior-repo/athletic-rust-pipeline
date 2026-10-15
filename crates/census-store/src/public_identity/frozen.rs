use crate::{AcceptedPublicIdentitySnapshot, StoreResult};
use sha2::{Digest, Sha256};

mod capture;
mod replay;
#[cfg(test)]
mod tests;
mod wire;

pub(crate) use capture::capture;
pub const MAX_FROZEN_PUBLIC_IDENTITY_BYTES: usize = 128 * 1024 * 1024;

#[derive(Debug)]
pub struct FrozenPublicIdentityBundleV1 {
    bytes: Vec<u8>,
    digest: String,
}

impl FrozenPublicIdentityBundleV1 {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn replay(&self) -> StoreResult<AcceptedPublicIdentitySnapshot> {
        replay_frozen_public_identity_v1(&self.bytes, &self.digest)
    }
}

pub fn replay_frozen_public_identity_v1(
    bytes: &[u8],
    expected_digest: &str,
) -> StoreResult<AcceptedPublicIdentitySnapshot> {
    replay::replay(bytes, expected_digest, MAX_FROZEN_PUBLIC_IDENTITY_BYTES)
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
