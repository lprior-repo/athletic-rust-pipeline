mod commit;
mod request;
mod witness;

use census_domain::public_identity_history::{PublicAthleteId, RegistryHead};
pub use request::SourceCandidateMembershipRequestV1;
pub(super) use witness::SourceCandidateMembershipWitnessV1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceCandidateMembershipOutcomeV1 {
    owner: PublicAthleteId,
    head: RegistryHead,
    replayed: bool,
}

impl SourceCandidateMembershipOutcomeV1 {
    pub fn public_id(&self) -> &PublicAthleteId {
        &self.owner
    }
    pub fn head(&self) -> &RegistryHead {
        &self.head
    }
    pub fn replayed(&self) -> bool {
        self.replayed
    }
    pub fn introduced_public_ids(&self) -> &[PublicAthleteId] {
        &[]
    }
}

#[cfg(test)]
mod tests;
