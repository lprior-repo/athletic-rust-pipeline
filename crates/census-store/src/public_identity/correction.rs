mod commit;
mod request;
mod rows;
mod witness;

use census_domain::public_identity_history::{PublicAthleteId, RegistryHead};
pub use request::ReviewedIdentityRequestV1;
pub(super) use witness::ReviewedWitness;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedIdentityCommitOutcomeV1 {
    introduced_public_ids: Vec<PublicAthleteId>,
    head: RegistryHead,
    replayed: bool,
}

impl ReviewedIdentityCommitOutcomeV1 {
    pub fn introduced_public_ids(&self) -> &[PublicAthleteId] {
        &self.introduced_public_ids
    }
    pub fn head(&self) -> &RegistryHead {
        &self.head
    }
    pub fn replayed(&self) -> bool {
        self.replayed
    }
}

#[cfg(test)]
mod tests;
