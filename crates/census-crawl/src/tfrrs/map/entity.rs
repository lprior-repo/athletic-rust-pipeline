use super::state::{Absorb, AthleteFacts, Page, TeamFacts};
use census_domain::model::{
    AthleteId, CanonicalAthlete, CanonicalSchool, CanonicalTeam, Evidence, SchoolId,
    SourceIdentity, SourceNamespace, TeamId,
};

impl<'a> Absorb<'a> {
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
                retained_conflicts: Vec::new(),
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
    pub(super) fn athlete_for(
        &mut self,
        page: Page<'_>,
        facts: AthleteFacts<'_>,
    ) -> (AthleteId, SourceIdentity) {
        let source = facts.source;
        let id = CanonicalAthlete::mint(
            facts.school,
            facts.name,
            facts.grad_year,
            facts.gender,
            &source,
        );
        let athlete = self
            .accumulator
            .athletes
            .entry(id.as_str().to_string())
            .or_insert_with(|| {
                let mut athlete = CanonicalAthlete::new(
                    facts.school,
                    facts.name,
                    facts.grad_year,
                    facts.gender,
                    source.clone(),
                );
                athlete
                    .evidence
                    .push(Evidence::parsed(page.source.clone(), page.observed_on));
                athlete
            });
        if !athlete.sports.contains(&facts.sport) {
            athlete.sports.push(facts.sport);
        }
        if let Some(grade) = facts.observed_grade {
            if !athlete.observed_grades.contains(&grade) {
                athlete.observed_grades.push(grade);
            }
        }
        (id, source)
    }
}

pub(super) fn athlete_source(id: Option<u64>, source_key: &str) -> SourceIdentity {
    SourceIdentity {
        namespace: if id.is_some() {
            SourceNamespace::TfrrsAthlete
        } else {
            SourceNamespace::Other("tfrrs_document_row".to_string())
        },
        id: id.map_or_else(|| source_key.to_string(), |id| id.to_string()),
        url: None,
    }
}
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
