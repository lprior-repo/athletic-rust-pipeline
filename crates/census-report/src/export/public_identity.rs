use super::DatasetLineage;
use crate::report::{ReportError, ReportResult};
use census_domain::model::{
    AthleteCandidateId, AthleteIdentityProjection, CanonicalAthlete, StoreIdentity,
};
use census_domain::public_identity_history::{PublicAthleteId, PublicResolution, RegistryHead};
use census_domain::public_identity_qualification::PublicIdentityQualificationVersion;
use census_store::{AcceptedPublicIdentitySnapshot, StoreError};
use std::collections::{BTreeMap, HashMap};

pub(crate) mod output;
mod partition;
pub(crate) mod population;

const MAX_CANDIDATES: usize = 200_000;

pub(crate) struct VerifiedPublicIdentityView {
    accepted: AcceptedPublicIdentitySnapshot,
    input_generation: String,
    groups: BTreeMap<AthleteCandidateId, PublicIdentityGroup>,
    candidate_groups: BTreeMap<AthleteCandidateId, AthleteCandidateId>,
    aliases: HashMap<String, String>,
}

pub(crate) struct PublicIdentityGroup {
    canonical_candidate: AthleteCandidateId,
    public_id: Option<PublicAthleteId>,
    members: BTreeMap<AthleteCandidateId, PublicResolution>,
}

impl VerifiedPublicIdentityView {
    pub(super) fn capture(
        accepted: AcceptedPublicIdentitySnapshot,
        lineage: &DatasetLineage,
        retained_head: &RegistryHead,
        athletes: &[CanonicalAthlete],
        projection: &AthleteIdentityProjection,
    ) -> ReportResult<Self> {
        require_current_qualification(accepted.qualification_version())?;
        if accepted.store_identity().as_str() != lineage.store_identity
            || accepted.snapshot_sequence() != lineage.snapshot_sequence
            || accepted.head() != retained_head
        {
            return Err(invalid(
                "public identity view differs from the exact dataset lineage or registry head",
            ));
        }
        if athletes.len() > MAX_CANDIDATES {
            return Err(invalid(
                "public identity candidate inventory exceeds 200000 rows",
            ));
        }
        athletes.iter().try_for_each(|athlete| {
            projection
                .status(athlete.id.as_str())
                .map(|_| ())
                .map_err(StoreError::from)
        })?;
        let partition = partition::capture(&accepted, athletes)?;
        Ok(Self {
            accepted,
            input_generation: lineage.input_generation.clone(),
            groups: partition.groups,
            candidate_groups: partition.candidates,
            aliases: partition.aliases,
        })
    }

    pub(crate) fn input_generation(&self) -> &str {
        &self.input_generation
    }

    pub(crate) fn store_identity(&self) -> &StoreIdentity {
        self.accepted.store_identity()
    }

    #[cfg(test)]
    pub(crate) fn head(&self) -> &RegistryHead {
        self.accepted.head()
    }

    pub(crate) fn snapshot_sequence(&self) -> u64 {
        self.accepted.snapshot_sequence()
    }

    pub(crate) fn group(&self, candidate: &str) -> Option<&PublicIdentityGroup> {
        self.candidate_groups
            .get(candidate)
            .and_then(|canonical| self.groups.get(canonical))
    }

    pub(crate) fn candidate_resolution(&self, candidate: &str) -> Option<&PublicResolution> {
        self.group(candidate)
            .and_then(|group| group.members.get(candidate))
    }

    pub(crate) fn public_id(&self, candidate: &str) -> Option<&PublicAthleteId> {
        self.group(candidate)
            .and_then(PublicIdentityGroup::public_id)
    }

    pub(crate) fn aliases(&self) -> &HashMap<String, String> {
        &self.aliases
    }

    #[cfg(test)]
    pub(crate) fn public_ids(&self) -> impl Iterator<Item = &PublicAthleteId> {
        self.accepted.public_ids()
    }

    #[cfg(test)]
    pub(crate) fn resolve_public(
        &self,
        public_id: &PublicAthleteId,
    ) -> ReportResult<PublicResolution> {
        self.accepted.resolve_public(public_id).map_err(Into::into)
    }

    #[cfg(test)]
    pub(crate) fn correction_history(&self, public_id: &PublicAthleteId) -> Option<&[u64]> {
        self.accepted.correction_history(public_id)
    }

    #[cfg(test)]
    pub(crate) fn affected_record_keys(&self, sequence: u64) -> Option<&[String]> {
        self.accepted.affected_record_keys(sequence)
    }
}

impl PublicIdentityGroup {
    pub(crate) fn canonical_candidate(&self) -> &AthleteCandidateId {
        &self.canonical_candidate
    }

    pub(crate) fn public_id(&self) -> Option<&PublicAthleteId> {
        self.public_id.as_ref()
    }

    #[cfg(test)]
    pub(crate) fn members(&self) -> impl Iterator<Item = (&AthleteCandidateId, &PublicResolution)> {
        self.members.iter()
    }
}

fn inventory(athletes: &[CanonicalAthlete]) -> ReportResult<BTreeMap<&str, AthleteCandidateId>> {
    athletes
        .iter()
        .try_fold(BTreeMap::new(), |mut inventory, athlete| {
            if inventory
                .insert(athlete.id.as_str(), athlete.id.cast())
                .is_some()
            {
                return Err(invalid(
                    "public identity candidate inventory repeats a subject",
                ));
            }
            Ok(inventory)
        })
}

#[cfg(test)]
fn shared_public_id<'a>(
    mut members: impl Iterator<Item = &'a PublicResolution>,
) -> Option<&'a PublicAthleteId> {
    let first = resolved_id(members.next()?)?;
    members
        .all(|resolution| resolved_id(resolution) == Some(first))
        .then_some(first)
}

fn resolved_id(resolution: &PublicResolution) -> Option<&PublicAthleteId> {
    match resolution {
        PublicResolution::Canonical(identity)
        | PublicResolution::Redirect {
            successor: identity,
            ..
        }
        | PublicResolution::RevokedRedirect {
            successor: identity,
            ..
        } => Some(&identity.public_id),
        PublicResolution::Unknown
        | PublicResolution::Unresolved
        | PublicResolution::Ambiguous { .. } => None,
    }
}

fn invalid(detail: &str) -> ReportError {
    ReportError::Invariant {
        detail: detail.into(),
    }
}

fn require_current_qualification(version: PublicIdentityQualificationVersion) -> ReportResult<()> {
    if version != PublicIdentityQualificationVersion::RetainedAuthorityContextV2 {
        return Err(invalid(
            "public identity publication requires retained-authority qualification V2",
        ));
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests;
