use super::{HeldGraphWitness, Payload};
use crate::public_identity::{
    correction::ReviewedIdentityCommitOutcomeV1,
    local_adjudication::LocalAdjudicationCommitOutcomeV1,
    membership::SourceCandidateMembershipOutcomeV1,
    membership_withholding::MembershipWithholdingOutcomeV1, operation::Replayed,
    IdentityCommitOutcomeV1, PublicIdentityError,
};
use crate::StoreResult;
use census_domain::public_identity_history::{
    prepare_next, HistoryRecord, PublicAthleteId, RegistryHead,
};
impl HeldGraphWitness {
    pub(in crate::public_identity) fn outcome(
        &self,
        record: &HistoryRecord,
        replayed: bool,
    ) -> StoreResult<Replayed> {
        if self.record_digest != record.digest {
            return Err(PublicIdentityError::Corrupt("graph outcome record binding").into());
        }
        let head = RegistryHead {
            sequence: record.sequence,
            digest: record.digest.clone(),
        };
        Ok(match &self.payload {
            Payload::Source { public_id, .. } => Replayed::Source(IdentityCommitOutcomeV1 {
                public_id: PublicAthleteId::parse(public_id).map_err(PublicIdentityError::from)?,
                head,
                replayed,
            }),
            Payload::Reviewed {
                introduced_public_ids,
                ..
            } => Replayed::Reviewed(ReviewedIdentityCommitOutcomeV1 {
                introduced_public_ids: introduced_public_ids
                    .iter()
                    .map(|id| PublicAthleteId::parse(id).map_err(PublicIdentityError::from))
                    .collect::<Result<Vec<_>, _>>()?,
                head,
                replayed,
            }),
            Payload::Membership { request, .. } => {
                Replayed::Membership(SourceCandidateMembershipOutcomeV1 {
                    owner: request.owner.clone(),
                    head,
                    replayed,
                })
            }
            Payload::Manual { .. } => Replayed::Manual(LocalAdjudicationCommitOutcomeV1 {
                introduced_public_ids: prepare_next(
                    &record.installation,
                    &record.previous,
                    record.request.clone(),
                    crate::public_identity::REPLAY_LIMITS,
                )
                .map_err(PublicIdentityError::from)?
                .established_ids,
                head,
                replayed,
            }),
            Payload::Withholding { adjudication, .. } => {
                Replayed::MembershipWithheld(MembershipWithholdingOutcomeV1 {
                    head,
                    withheld_memberships: adjudication
                        .request
                        .expected_withheld_memberships
                        .clone(),
                    replayed,
                })
            }
        })
    }
}
