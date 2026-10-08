use super::map::{EventContext, Mapper, PerformanceRow, HIGH_SCHOOL};
use super::wire::TeamRef;
use census_domain::model::{
    AthleteId, CanonicalPerformance, CanonicalTeam, Gender, SchoolId, SchoolYear, SourceIdentity,
    SourceNamespace, Sport, TeamId,
};

mod athletes;

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
            &context.event.id,
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
            source_athlete: Some(subject.1),
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
