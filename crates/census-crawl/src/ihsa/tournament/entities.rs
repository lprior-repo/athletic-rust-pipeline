use super::map::{AthleteRow, EventContext, Mapper, PerformanceRow, ASSOCIATION, HIGH_SCHOOL};
use super::wire::TeamRef;
use census_domain::model::{
    AthleteId, CanonicalAthlete, CanonicalPerformance, CanonicalTeam, Evidence, Gender,
    ObservedGrade, SchoolId, SchoolYear, SourceIdentity, SourceNamespace, Sport, TeamId,
};

impl<'a> Mapper<'a> {
    pub(super) fn team(
        &mut self,
        school: &SchoolId,
        sport: Sport,
        gender: Gender,
        school_year: SchoolYear,
        reference: Option<&TeamRef>,
        url: &str,
    ) -> TeamId {
        let id = CanonicalTeam::mint(school, sport, gender, school_year);
        let identities = team_identities(reference);
        let evidence = self.origin.evidence(url);
        let key = id.as_str().to_string();
        if let Some(team) = self.accumulated.teams.get_mut(&key) {
            push_all(&mut team.source_identities, identities);
            push_once(&mut team.evidence, evidence);
            return id;
        }
        let team = CanonicalTeam {
            id: id.clone(),
            school: school.clone(),
            sport,
            gender,
            school_year,
            level: Some(HIGH_SCHOOL.to_string()),
            source_identities: identities,
            evidence: vec![evidence],
            retained_conflicts: Vec::new(),
        };
        self.accumulated.teams.insert(key, team);
        id
    }

    pub(super) fn athlete(
        &mut self,
        row: AthleteRow<'_>,
        evidence: Evidence,
    ) -> Option<(AthleteId, SourceIdentity)> {
        let name = row.name.map(str::trim).filter(|value| !value.is_empty())?;
        let grade = row.grade?;
        let observation = ObservedGrade {
            grade,
            school_year: row.school_year,
            source: evidence.source.clone(),
        };
        let identities = athlete_identities(row.net_id, row.live_id, row.entry.as_deref());
        let grad_year = observation.grad_year();
        let source = identities.first().cloned().map_or(
            SourceIdentity::new(
                SourceNamespace::Other("ihsa_result_row".to_string()),
                row.source_key,
            ),
            |source| source,
        );
        let id = CanonicalAthlete::mint(row.school, name, grad_year, row.gender, &source);
        let key = id.as_str().to_string();
        if let Some(athlete) = self.accumulated.athletes.get_mut(&key) {
            push_once(&mut athlete.observed_grades, observation);
            push_once(&mut athlete.sports, row.sport);
            for identity in identities {
                athlete.add_identity(identity);
            }
            push_once(&mut athlete.evidence, evidence);
            return Some((athlete.id.clone(), source.clone()));
        }
        let mut athlete = CanonicalAthlete::new(row.school, name, grad_year, row.gender, source);
        athlete.observed_grades.push(observation);
        athlete.sports.push(row.sport);
        for identity in identities {
            athlete.add_identity(identity);
        }
        athlete.evidence.push(evidence);
        let subject = (athlete.id.clone(), athlete.source.clone());
        self.accumulated.athletes.insert(key, athlete);
        Some(subject)
    }

    pub(super) fn performance(
        &mut self,
        subject: (AthleteId, SourceIdentity),
        team: &TeamId,
        context: &EventContext<'_>,
        row: PerformanceRow<'_>,
        url: &str,
    ) {
        let id = CanonicalPerformance::mint(
            &subject.0,
            &context.meet.id,
            &context.event.kind,
            row.date,
            &row.source_key,
        );
        let key = id.as_str().to_string();
        if self.accumulated.performances.contains_key(&key) {
            return;
        }
        let performance = CanonicalPerformance {
            id,
            athlete: subject.0.clone(),
            team: team.clone(),
            event: context.event.id.clone(),
            meet: context.meet.id.clone(),
            date: row.date.to_string(),
            mark: row.mark,
            wind_mps: None,
            place: row.place,
            heat: None,
            round: context.event.round.clone(),
            timing: None,
            observed_grade: row.grade,
            evidence: vec![self.origin.evidence(url)],
            source_key: row.source_key,
            source_athlete: subject.1,
            retained_conflicts: Vec::new(),
        };
        self.accumulated.performances.insert(key, performance);
        self.stats.performances = self.stats.performances.saturating_add(1);
    }
}

fn team_identities(reference: Option<&TeamRef>) -> Vec<SourceIdentity> {
    let Some(team) = reference else {
        return Vec::new();
    };
    let mut identities = Vec::new();
    if let Some(id) = team.athletic_net_id {
        identities.push(SourceIdentity::new(
            SourceNamespace::AthleticNet {
                kind: "team".to_string(),
            },
            id.to_string(),
        ));
    }
    if let Some(id) = team.athletic_live_id {
        identities.push(SourceIdentity::new(
            SourceNamespace::AthleticNet {
                kind: "live".to_string(),
            },
            id.to_string(),
        ));
    }
    identities
}

fn athlete_identities(
    net_id: Option<u64>,
    live_id: Option<u64>,
    entry: Option<&str>,
) -> Vec<SourceIdentity> {
    if let Some(entry) = entry {
        return vec![SourceIdentity::new(
            SourceNamespace::AssociationAthlete {
                association: ASSOCIATION.to_string(),
            },
            entry,
        )];
    }
    let mut identities = Vec::new();
    if let Some(id) = net_id {
        identities.push(SourceIdentity::new(
            SourceNamespace::AthleticNet {
                kind: "athlete".to_string(),
            },
            id.to_string(),
        ));
    }
    if let Some(id) = live_id {
        identities.push(SourceIdentity::new(
            SourceNamespace::AthleticNet {
                kind: "live".to_string(),
            },
            id.to_string(),
        ));
    }
    identities
}

fn push_once<T: PartialEq>(items: &mut Vec<T>, item: T) {
    if !items.contains(&item) {
        items.push(item);
    }
}

fn push_all<T: PartialEq + Clone>(items: &mut Vec<T>, more: Vec<T>) {
    for item in more {
        push_once(items, item);
    }
}
