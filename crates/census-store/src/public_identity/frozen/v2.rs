use crate::{AcceptedPublicIdentitySnapshot, StoreResult};

mod capture;
mod measure;
mod replay;
mod retained;
#[cfg(test)]
mod tests;
mod walk;
mod wire;

pub(crate) use capture::capture;
pub(crate) use measure::measure;
pub use measure::{FrozenPublicIdentityDemandV2, MAX_FROZEN_PUBLIC_IDENTITY_DEMAND_JSON_BYTES};

#[derive(Debug)]
pub struct FrozenPublicIdentityBundleV2 {
    bytes: Vec<u8>,
    digest: String,
}

impl FrozenPublicIdentityBundleV2 {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn replay(&self) -> StoreResult<AcceptedPublicIdentitySnapshot> {
        replay_frozen_public_identity_v2(&self.bytes, &self.digest)
    }
}

pub fn replay_frozen_public_identity_v2(
    bytes: &[u8],
    expected_digest: &str,
) -> StoreResult<AcceptedPublicIdentitySnapshot> {
    replay::replay(bytes, expected_digest)
}
