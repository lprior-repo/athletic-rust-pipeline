use crate::public_identity::PublicIdentityError;
use crate::{StoreError, StoreResult};
use census_domain::model::{AthleteCandidateId, CanonicalAthlete};
use census_domain::public_identity_history::{
    HistoryProjection, PublicAthleteId, PublicResolution, ReplayLimits,
};

pub(in crate::public_identity) struct Members<'a> {
    projection: &'a HistoryProjection,
    canonical: &'a PublicAthleteId,
    members: Vec<AthleteCandidateId>,
}
impl<'a> Members<'a> {
    pub(in crate::public_identity) fn new(
        projection: &'a HistoryProjection,
        canonical: &'a PublicAthleteId,
    ) -> Self {
        Self {
            projection,
            canonical,
            members: Vec::new(),
        }
    }
    pub(in crate::public_identity) fn observe(
        &mut self,
        athlete: &CanonicalAthlete,
    ) -> StoreResult<()> {
        let candidate = athlete.id.cast();
        let current = match self
            .projection
            .resolve_candidate(&candidate)
            .map_err(PublicIdentityError::from)?
        {
            PublicResolution::Canonical(row)
            | PublicResolution::Redirect { successor: row, .. }
            | PublicResolution::RevokedRedirect { successor: row, .. } => row.public_id,
            _ => return Ok(()),
        };
        if &current != self.canonical {
            return Ok(());
        }
        if self.members.len() >= ReplayLimits::SMALL.members_per_operation {
            return Err(PublicIdentityError::Corrupt("manual member bound").into());
        }
        self.members
            .try_reserve(1)
            .map_err(|error| StoreError::Refused {
                detail: format!("manual member allocation: {error}"),
            })?;
        self.members.push(candidate);
        Ok(())
    }
    pub(in crate::public_identity) fn finish(mut self) -> Vec<AthleteCandidateId> {
        self.members.sort();
        self.members.dedup();
        self.members
    }
}
