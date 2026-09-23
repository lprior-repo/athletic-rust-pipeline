//! Canonical entities: the teams, athletes and performances a run's rows mint.
//!
//! Every id the payload publishes is kept as a [`SourceIdentity`] and nothing else is: the team's
//! `athleticNetId`, the athlete's `athleticNetId`/`athleticLiveId`, and the association's own entry
//! number where the list publishes one. Two rows that agree on the same published id upsert one
//! entity; a row with no published id falls back to the domain's natural key (school + normalized
//! name + graduating class + gender for an athlete), which is why a grade-less row mints no athlete.
//!
//! [`SourceIdentity`]: census_domain::model::SourceIdentity

use super::map::{AthleteRow, EventContext, Mapper, PerformanceRow, ASSOCIATION, HIGH_SCHOOL};
use super::wire::TeamRef;
use census_domain::model::{
    AthleteId, CanonicalAthlete, CanonicalPerformance, CanonicalTeam, Evidence, Gender,
    ObservedGrade, SchoolId, SchoolYear, SourceIdentity, SourceNamespace, Sport, TeamId,
};

impl<'a> Mapper<'a> {
    /// Mint or find one team on the platform's (school, sport, gender, season) key.
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

    /// Mint or find one athlete, accumulating the observations every further row adds.
    ///
    /// `None` when the row publishes no name or no grade: the graduating class is part of the natural
    /// key, so an athlete cannot be minted from a row that does not state it.
    pub(super) fn athlete(&mut self, row: AthleteRow<'_>, evidence: Evidence) -> Option<AthleteId> {
        let name = row.name.map(str::trim).filter(|value| !value.is_empty())?;
        let grade = row.grade?;
        let observation = ObservedGrade {
            grade,
            school_year: row.school_year,
            source: evidence.source.clone(),
        };
        let identities = athlete_identities(row.net_id, row.live_id, row.entry.as_deref());
        let grad_year = observation.grad_year();
        let key = row.net_id.map_or_else(
            || {
                CanonicalAthlete::mint(row.school, name, grad_year, row.gender)
                    .as_str()
                    .to_string()
            },
            |id| format!("net:{id}"),
        );
        if let Some(athlete) = self.accumulated.athletes.get_mut(&key) {
            push_once(&mut athlete.observed_grades, observation);
            push_once(&mut athlete.sports, row.sport);
            push_all(&mut athlete.source_identities, identities);
            push_once(&mut athlete.evidence, evidence);
            return Some(athlete.id.clone());
        }
        let mut athlete = CanonicalAthlete::new(row.school, name, grad_year, row.gender);
        athlete.sports.push(row.sport);
        athlete.observed_grades.push(observation);
        athlete.source_identities.extend(identities);
        athlete.evidence.push(evidence);
        let id = athlete.id.clone();
        self.accumulated.athletes.insert(key, athlete);
        Some(id)
    }

    /// Store one performance against the event its own round belongs to.
    ///
    /// A performance the run already minted is left alone: the identifier is
    /// (athlete, meet, event kind, date, source key), so a re-read row upserts rather than doubles.
    pub(super) fn performance(
        &mut self,
        athlete: &AthleteId,
        team: &TeamId,
        context: &EventContext<'_>,
        row: PerformanceRow<'_>,
        url: &str,
    ) {
        let id = CanonicalPerformance::mint(
            athlete,
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
            athlete: athlete.clone(),
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
            retained_conflicts: Vec::new(),
        };
        self.accumulated.performances.insert(key, performance);
        self.stats.performances = self.stats.performances.saturating_add(1);
    }
}

/// The Athletic.net identity a team row publishes, net id and Live id each under its own kind.
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

/// The identity an athlete row publishes: the association's own entry number when it has one,
/// otherwise the Athletic.net net id and Live id.
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

/// Append `item` unless the vector already carries it.
///
/// The store merges duplicate observations anyway; this keeps one run's entity small instead of
/// repeating the same evidence for every event the athlete took part in.
fn push_once<T: PartialEq>(items: &mut Vec<T>, item: T) {
    if !items.contains(&item) {
        items.push(item);
    }
}

/// Append every item the vector does not carry yet.
fn push_all<T: PartialEq + Clone>(items: &mut Vec<T>, more: Vec<T>) {
    for item in more {
        push_once(items, item);
    }
}
