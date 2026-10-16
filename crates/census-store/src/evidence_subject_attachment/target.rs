use super::encoding::{self, EncodedObject};
use super::EvidenceSubjectAttachmentStoreError as Error;
use crate::{AcceptedPublicIdentitySnapshot, StoreResult};
use census_domain::evidence_subject_attachment::{
    AttachmentObjectReferenceV1, AttachmentReplayGrantV1, AttachmentTargetV1,
    EvidenceSubjectAttachmentError,
};
use census_domain::model::{AthleteCandidateId, StoreIdentity};
use census_domain::public_identity_history::{
    BindingClaim, CandidateMembershipClaimV1, EstablishmentOrdinal, PublicAthleteId,
    PublicResolution, RegistryHead, ReplayLimits, ResolvedSuccessor,
};
use census_domain::public_identity_qualification::PublicIdentityQualificationVersion;
use serde::Serialize;

mod group;
mod historical;
mod historical_claims;
pub(super) use historical::verify as verify_historical;
mod witness;

pub(super) use witness::{encode_witness, WitnessInputs};

pub(super) struct CurrentTarget<'a> {
    accepted: &'a AcceptedPublicIdentitySnapshot,
    resolved: ResolvedSuccessor,
    group: EncodedObject,
}

impl CurrentTarget<'_> {
    pub(super) fn into_group(self) -> EncodedObject {
        self.group
    }
    pub(super) fn group(&self) -> &EncodedObject {
        &self.group
    }
    pub(super) fn resolved(&self) -> &ResolvedSuccessor {
        &self.resolved
    }

    pub(super) fn require_command_target(
        &self,
        target: &AttachmentTargetV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<()> {
        grant.charge_steps(1).map_err(Error::Domain)?;
        if self.resolved.public_id != target.public_id
            || self.resolved.representative_candidate != target.canonical_candidate
            || self.group.reference().digest != target.accepted_group_digest
        {
            grant.poison();
            return Err(Error::TargetMoved.into());
        }
        Ok(())
    }
}

pub(super) fn current<'a>(
    accepted: &'a AcceptedPublicIdentitySnapshot,
    target: &PublicAthleteId,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<CurrentTarget<'a>> {
    grant.ensure_available().map_err(Error::Domain)?;
    let result = group::current(accepted, target, grant);
    if result.is_err() {
        grant.poison();
    }
    result
}

#[cfg(test)]
mod tests;
