use super::AcceptedPublicIdentitySnapshot;
use crate::{PublicIdentityError, StoreResult};
use census_domain::public_identity_history::{
    BindingClaim, CandidateMembershipClaimV1, PublicAthleteId,
};
use std::collections::BTreeSet;

impl AcceptedPublicIdentitySnapshot {
    pub fn component_bindings(&self, canonical: &PublicAthleteId) -> StoreResult<BTreeSet<String>> {
        self.history
            .projection()
            .component_bindings(canonical)
            .map_err(|error| PublicIdentityError::from(error).into())
    }

    pub fn binding_claim(&self, id: &str) -> Option<&BindingClaim> {
        self.history.projection().binding_claim(id)
    }

    pub fn membership_claim(&self, id: &str) -> Option<&CandidateMembershipClaimV1> {
        self.history.projection().membership_claim(id)
    }
}
