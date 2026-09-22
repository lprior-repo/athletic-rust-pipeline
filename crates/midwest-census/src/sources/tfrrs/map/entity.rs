//! The mints for the three entities a row's own text names: its school, its team, its athlete.
//!
//! Each one mints a canonical id from the domain's own key and records the provider's channel
//! beside it (`TfrrsTeam`, `TfrrsAthlete`), so a second page that names the same entity upserts
//! it instead of adding a twin.

use super::state::{Absorb, AthleteFacts, Page, TeamFacts};
use census_domain::model::{
    AthleteId, CanonicalAthlete, CanonicalSchool, CanonicalTeam, Evidence, SchoolId,
    SourceIdentity, SourceNamespace, TeamId,
};

impl<'a> Absorb<'a> {
    /// Resolve a school against the consolidated index, memoized per run, minting it when the index
    /// has never seen it.
    ///
    /// School identity keys on the page's own state + the normalized name, so the mint is
    /// deterministic and a second run over the same list resolves to the same school instead of
    /// adding a twin.
    pub(super) fn school_for(&mut self, page: Page<'_>, name: &str) -> Option<SchoolId> {
        if let Some(id) = self.resolved.get(name) {
            return Some(id.clone());
        }
        if let Some((id, _)) = self.index.resolve(page.jurisdiction, name) {
            self.stats.schools_resolved = self.stats.schools_resolved.saturating_add(1);
            self.resolved.insert(name.to_string(), id.clone());
            return Some(id);
        }
        let (school, id) = CanonicalSchool::new(page.jurisdiction, name, name.to_lowercase());
        self.stats.schools_minted = self.stats.schools_minted.saturating_add(1);
        self.accumulator
            .schools
            .entry(id.as_str().to_string())
            .or_insert(school);
        self.resolved.insert(name.to_string(), id.clone());
        Some(id)
    }

    /// Resolve or mint the team an observation names.
    ///
    /// Team identity is (school, sport, gender, school year): the slug is an identity *channel*, so a
    /// roster and a list that name the same team in the same season upsert one team.
    pub(super) fn team_for(&mut self, page: Page<'_>, facts: &TeamFacts<'_>) -> TeamId {
        let id = CanonicalTeam::mint(facts.school, facts.sport, facts.gender, facts.school_year);
        let team = self
            .accumulator
            .teams
            .entry(id.as_str().to_string())
            .or_insert_with(|| CanonicalTeam {
                id: id.clone(),
                school: facts.school.clone(),
                sport: facts.sport,
                gender: facts.gender,
                school_year: facts.school_year,
                level: Some("high_school".to_string()),
                source_identities: Vec::new(),
                evidence: vec![Evidence::parsed(page.source.clone(), page.observed_on)],
            });
        if let Some(slug) = facts.slug {
            push_identity(
                &mut team.source_identities,
                SourceNamespace::TfrrsTeam,
                slug,
                facts.path.map(str::to_string),
            );
        }
        id
    }
    /// Resolve or mint one athlete, recording every channel the page published for them.
    pub(super) fn athlete_for(&mut self, page: Page<'_>, facts: &AthleteFacts<'_>) -> AthleteId {
        let id = CanonicalAthlete::mint(facts.school, facts.name, facts.grad_year, facts.gender);
        let athlete = self
            .accumulator
            .athletes
            .entry(id.as_str().to_string())
            .or_insert_with(|| {
                let mut athlete =
                    CanonicalAthlete::new(facts.school, facts.name, facts.grad_year, facts.gender);
                athlete
                    .evidence
                    .push(Evidence::parsed(page.source.clone(), page.observed_on));
                athlete
            });
        if !athlete.sports.contains(&facts.sport) {
            athlete.sports.push(facts.sport);
        }
        if let Some(grade) = facts.observed_grade.as_ref() {
            if !athlete.observed_grades.contains(grade) {
                athlete.observed_grades.push(grade.clone());
            }
        }
        if let Some(numeric) = facts.tfrrs_id {
            push_identity(
                &mut athlete.source_identities,
                SourceNamespace::TfrrsAthlete,
                &numeric.to_string(),
                facts.url.clone(),
            );
        }
        id
    }
}
/// Push an identity channel when the same namespace + id is not already recorded.
pub(super) fn push_identity(
    identities: &mut Vec<SourceIdentity>,
    namespace: SourceNamespace,
    id: &str,
    url: Option<String>,
) {
    let known = identities
        .iter()
        .any(|identity| identity.namespace == namespace && identity.id == id);
    if !known {
        let mut identity = SourceIdentity::new(namespace, id);
        if let Some(url) = url {
            identity = identity.with_url(url);
        }
        identities.push(identity);
    }
}
