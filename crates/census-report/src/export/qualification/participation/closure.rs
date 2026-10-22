use super::{
    contexts::{Entity, ResultContexts},
    ParticipationDiagnostic as Gap,
};
use crate::export::participation_bridge::SourceCohortAgreement;
use crate::export::public_identity::population::{IdentityAdmission, IdentityPopulation};
use crate::export::ExportDataset;
use census_domain::model::{
    serialized_digest, CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalTeam, Evidence,
};
use census_store::{StoreError, StoreResult};
use std::collections::BTreeSet;

mod index;

const MAX_EVIDENCE_REFERENCES: u64 = 65_536;

#[cfg(test)]
mod tests;

pub(super) struct Closure {
    pub(super) athletes: Vec<CanonicalAthlete>,
    pub(super) performances: Vec<CanonicalPerformance>,
    pub(super) teams: Vec<CanonicalTeam>,
    pub(super) meets: Vec<CanonicalMeet>,
    pub(super) events: Vec<CanonicalEvent>,
}

pub(super) struct IndexedClosure<'a> {
    index: index::Index<'a>,
}

impl Closure {
    pub(super) fn indexed<'a>(
        &'a self,
        dataset: &'a ExportDataset,
        identities: &IdentityPopulation<'_>,
    ) -> IndexedClosure<'a> {
        IndexedClosure {
            index: index::Index::new(self, dataset, identities),
        }
    }
}

impl<'raw> IndexedClosure<'raw> {
    pub(super) fn cohort(
        &self,
        identities: &IdentityPopulation<'_>,
        performance: &CanonicalPerformance,
    ) -> SourceCohortAgreement {
        self.index
            .athletes(identities, performance.athlete.as_str())
            .into_iter()
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
        identities: &IdentityPopulation<'_>,
        performance: &CanonicalPerformance,
        contexts: &ResultContexts,
        roster_context: super::VerifiedRosterContext<'_>,
    ) -> StoreResult<BTreeSet<Gap>> {
        let mut gaps = BTreeSet::new();
        if identities.admission(performance.athlete.as_str()) != IdentityAdmission::Accepted {
            gaps.insert(Gap::IdentityUnaccepted);
        }
        if !self.index.source_bound(identities, performance) {
            gaps.insert(Gap::SourceBindingMismatch);
        }
        if self
            .performances(performance)
            .any(|row| !row.retained_conflicts.is_empty())
        {
            gaps.insert(Gap::CanonicalConflict);
        }
        if !performance.retained_conflicts.is_empty() {
            gaps.insert(Gap::CanonicalConflict);
        }
        if self
            .index
            .athletes(identities, performance.athlete.as_str())
            .into_iter()
            .any(|athlete| !athlete.retained_conflicts.is_empty())
        {
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
            .try_for_each(|(_, evidence)| retain_reference(references, evidence))
    }

    fn evidence<'a, 'i>(
        &'a self,
        identities: &'a IdentityPopulation<'i>,
        performance: &'a CanonicalPerformance,
    ) -> impl Iterator<Item = (Entity<'a>, &'a Evidence)> + use<'a, 'i, 'raw> {
        let teams = self
            .index
            .teams(self.versions(performance).map(|row| &row.team));
        let meets = self
            .index
            .meets(self.versions(performance).map(|row| &row.meet));
        let events = self
            .index
            .events(self.versions(performance).map(|row| &row.event));
        self.index
            .athletes(identities, performance.athlete.as_str())
            .into_iter()
            .flat_map(|row| tagged(Entity::Athlete(&row.id), &row.evidence))
            .chain(
                self.performances(performance)
                    .flat_map(|row| tagged(Entity::Performance(&row.id), &row.evidence)),
            )
            .chain(
                teams
                    .into_iter()
                    .flat_map(|row| tagged(Entity::Team(&row.id), &row.evidence)),
            )
            .chain(
                meets
                    .into_iter()
                    .flat_map(|row| tagged(Entity::Meet(&row.id), &row.evidence)),
            )
            .chain(
                events
                    .into_iter()
                    .flat_map(|row| tagged(Entity::Event(&row.id), &row.evidence)),
            )
    }
    fn versions<'a>(
        &'a self,
        performance: &'a CanonicalPerformance,
    ) -> impl Iterator<Item = &'a CanonicalPerformance> {
        self.performances(performance)
            .chain(std::iter::once(performance))
    }

    pub(super) fn performances<'a>(
        &'a self,
        performance: &CanonicalPerformance,
    ) -> std::vec::IntoIter<&'a CanonicalPerformance> {
        self.index.performances(&performance.id).into_iter()
    }

    fn related_conflict(&self, performance: &CanonicalPerformance) -> bool {
        self.versions(performance).any(|version| {
            let teams = self.index.team(&version.team);
            let meets = self.index.meet(&version.meet);
            let events = self.index.event(&version.event);
            teams.is_empty()
                || meets.is_empty()
                || events.is_empty()
                || teams.iter().any(|row| !row.retained_conflicts.is_empty())
                || meets.iter().any(|row| !row.retained_conflicts.is_empty())
                || events.iter().any(|row| !row.retained_conflicts.is_empty())
        })
    }
}

fn tagged<'a>(
    entity: Entity<'a>,
    evidence: &'a [Evidence],
) -> impl Iterator<Item = (Entity<'a>, &'a Evidence)> {
    evidence.iter().map(move |evidence| (entity, evidence))
}

fn retain_reference(references: &mut BTreeSet<String>, evidence: &Evidence) -> StoreResult<()> {
    let reference = format!("canonical_evidence:{}", serialized_digest(evidence)?);
    if references.contains(&reference) {
        return Ok(());
    }
    let needed = u64::try_from(references.len())
        .map_err(|_| StoreError::CounterOverflow)?
        .checked_add(1)
        .ok_or(StoreError::CounterOverflow)?;
    if needed > MAX_EVIDENCE_REFERENCES {
        return Err(StoreError::BudgetExceeded {
            what: "participation evidence closure distinct proofs",
            needed,
            available: MAX_EVIDENCE_REFERENCES,
        });
    }
    references.insert(reference);
    Ok(())
}
