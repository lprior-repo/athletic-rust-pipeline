use super::{
    contexts::{Entity, ResultContexts},
    invalid, ParticipationDiagnostic as Gap,
};
use crate::export::participation_bridge::SourceCohortAgreement;
use crate::export::public_identity::population::{IdentityAdmission, IdentityPopulation};
use crate::export::ExportDataset;
use census_domain::model::{
    serialized_digest, CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalTeam, Evidence,
};
use census_store::StoreResult;
use std::collections::BTreeSet;

pub(super) struct Closure {
    pub(super) athletes: Vec<CanonicalAthlete>,
    pub(super) performances: Vec<CanonicalPerformance>,
    pub(super) teams: Vec<CanonicalTeam>,
    pub(super) meets: Vec<CanonicalMeet>,
    pub(super) events: Vec<CanonicalEvent>,
}

impl Closure {
    pub(super) fn cohort(
        &self,
        identities: &IdentityPopulation<'_>,
        performance: &CanonicalPerformance,
    ) -> SourceCohortAgreement {
        self.athletes
            .iter()
            .filter(|athlete| {
                identities.same_group(athlete.id.as_str(), performance.athlete.as_str())
            })
            .fold(SourceCohortAgreement::Missing, |held, athlete| {
                if athlete.has_cohort_conflict() {
                    return SourceCohortAgreement::Conflicting;
                }
                match held {
                    SourceCohortAgreement::Missing => {
                        SourceCohortAgreement::Resolved(athlete.grad_year)
                    }
                    SourceCohortAgreement::Resolved(year) if year == athlete.grad_year => held,
                    _ => SourceCohortAgreement::Conflicting,
                }
            })
    }

    pub(super) fn gate(
        &self,
        dataset: &ExportDataset,
        identities: &IdentityPopulation<'_>,
        performance: &CanonicalPerformance,
        contexts: &ResultContexts,
        roster_context: super::VerifiedRosterContext<'_>,
    ) -> StoreResult<BTreeSet<Gap>> {
        let mut gaps = BTreeSet::new();
        if identities.admission(performance.athlete.as_str()) != IdentityAdmission::Accepted {
            gaps.insert(Gap::IdentityUnaccepted);
        }
        if performance.source_athlete.as_ref().is_none_or(|owner| {
            !dataset.athletes.iter().any(|athlete| {
                identities.same_group(athlete.id.as_str(), performance.athlete.as_str())
                    && identities.admission(athlete.id.as_str()) == IdentityAdmission::Accepted
                    && athlete.identities().any(|identity| {
                        identity.namespace == owner.namespace && identity.id == owner.id
                    })
            })
        }) {
            gaps.insert(Gap::SourceBindingMismatch);
        }
        if self
            .performances
            .iter()
            .any(|row| row.id == performance.id && !row.retained_conflicts.is_empty())
        {
            gaps.insert(Gap::CanonicalConflict);
        }
        if !performance.retained_conflicts.is_empty() {
            gaps.insert(Gap::CanonicalConflict);
        }
        if self.athletes.iter().any(|athlete| {
            identities.same_group(athlete.id.as_str(), performance.athlete.as_str())
                && !athlete.retained_conflicts.is_empty()
        }) {
            gaps.insert(Gap::CanonicalConflict);
        }
        if self.related_conflict(performance) {
            gaps.insert(Gap::CanonicalConflict);
        }
        if identities
            .canonical_candidate(performance.athlete.as_str())
            .is_some_and(|candidate| roster_context.conflicted(candidate))
        {
            gaps.insert(Gap::CanonicalConflict);
        }
        if self.has_unverified_context(identities, performance, contexts, roster_context)? {
            gaps.insert(Gap::UnverifiedAdditionalEvidence);
        }
        Ok(gaps)
    }

    fn has_unverified_context(
        &self,
        identities: &IdentityPopulation<'_>,
        performance: &CanonicalPerformance,
        contexts: &ResultContexts,
        roster_context: super::VerifiedRosterContext<'_>,
    ) -> StoreResult<bool> {
        self.evidence(identities, performance)
            .try_fold(false, |unverified, (entity, evidence)| {
                let roster_supported = match entity {
                    Entity::Athlete(athlete) => roster_context.contains(athlete, evidence)?,
                    Entity::Performance(_)
                    | Entity::Team(_)
                    | Entity::Meet(_)
                    | Entity::Event(_) => false,
                };
                let supported = contexts.contains(entity, evidence)? || roster_supported;
                Ok(unverified || !supported)
            })
    }

    pub(super) fn references(
        &self,
        identities: &IdentityPopulation<'_>,
        performance: &CanonicalPerformance,
        references: &mut BTreeSet<String>,
    ) -> StoreResult<()> {
        self.evidence(identities, performance)
            .try_for_each(|(_, evidence)| {
                references.insert(format!(
                    "canonical_evidence:{}",
                    serialized_digest(evidence)?
                ));
                if references.len() > 256 {
                    return Err(invalid(
                        "participation evidence closure exceeds 256 distinct proofs",
                    ));
                }
                Ok(())
            })
    }

    fn evidence<'a, 'i>(
        &'a self,
        identities: &'a IdentityPopulation<'i>,
        performance: &'a CanonicalPerformance,
    ) -> impl Iterator<Item = (Entity<'a>, &'a Evidence)> + use<'a, 'i> {
        let teams: BTreeSet<_> = self.versions(performance).map(|row| &row.team).collect();
        let meets: BTreeSet<_> = self.versions(performance).map(|row| &row.meet).collect();
        let events: BTreeSet<_> = self.versions(performance).map(|row| &row.event).collect();
        self.athletes
            .iter()
            .filter(move |athlete| {
                identities.same_group(athlete.id.as_str(), performance.athlete.as_str())
            })
            .flat_map(|row| tagged(Entity::Athlete(&row.id), &row.evidence))
            .chain(
                self.performances
                    .iter()
                    .filter(|row| row.id == performance.id)
                    .flat_map(|row| tagged(Entity::Performance(&row.id), &row.evidence)),
            )
            .chain(
                self.teams
                    .iter()
                    .filter(move |row| teams.contains(&row.id))
                    .flat_map(|row| tagged(Entity::Team(&row.id), &row.evidence)),
            )
            .chain(
                self.meets
                    .iter()
                    .filter(move |row| meets.contains(&row.id))
                    .flat_map(|row| tagged(Entity::Meet(&row.id), &row.evidence)),
            )
            .chain(
                self.events
                    .iter()
                    .filter(move |row| events.contains(&row.id))
                    .flat_map(|row| tagged(Entity::Event(&row.id), &row.evidence)),
            )
    }
    fn versions<'a>(
        &'a self,
        performance: &'a CanonicalPerformance,
    ) -> impl Iterator<Item = &'a CanonicalPerformance> {
        self.performances
            .iter()
            .filter(|row| row.id == performance.id)
            .chain(std::iter::once(performance))
    }

    fn related_conflict(&self, performance: &CanonicalPerformance) -> bool {
        self.versions(performance).any(|version| {
            !self.teams.iter().any(|row| row.id == version.team)
                || !self.meets.iter().any(|row| row.id == version.meet)
                || !self.events.iter().any(|row| row.id == version.event)
                || self
                    .teams
                    .iter()
                    .any(|row| row.id == version.team && !row.retained_conflicts.is_empty())
                || self
                    .meets
                    .iter()
                    .any(|row| row.id == version.meet && !row.retained_conflicts.is_empty())
                || self
                    .events
                    .iter()
                    .any(|row| row.id == version.event && !row.retained_conflicts.is_empty())
        })
    }
}

fn tagged<'a>(
    entity: Entity<'a>,
    evidence: &'a [Evidence],
) -> impl Iterator<Item = (Entity<'a>, &'a Evidence)> {
    evidence.iter().map(move |evidence| (entity, evidence))
}
