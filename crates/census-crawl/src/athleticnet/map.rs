use super::parse::Bio;
use super::SCHOOL_KIND;
use census_domain::model::{
    AthleteId, CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam, CompetitionLevel, EventKind, Evidence, Gender, Grade, Mark,
    ObservedGrade, ReviewCase, SchoolId, SchoolYear, SourceIdentity, SourceNamespace,
    SourceObservation, SourceRef, Sport, TeamId, TimingMethod,
};
use census_domain::school_index::SchoolIndex;
use census_domain::UsJurisdiction;
use std::collections::HashMap;
pub(super) mod events;

pub(super) fn profile_url(athlete_id: u64) -> String {
    format!("https://www.athletic.net/athlete/{athlete_id}/track-and-field")
}

#[derive(Debug, Default)]
pub(super) struct Stats {
    pub(super) athletes_seen: u64,
    pub(super) athletes_absorbed: u64,
    pub(super) athletes_without_grade: u64,
    pub(super) athletes_without_school: u64,
    pub(super) gender_unknown: u64,
    pub(super) rows_seen: u64,
    pub(super) rows_absorbed: u64,
    pub(super) rows_no_mark: u64,
    pub(super) rows_no_season: u64,
    pub(super) rows_unknown_season: HashMap<String, u64>,
    pub(super) rows_no_event: u64,
    pub(super) rows_unknown_school: u64,
    pub(super) rows_unknown_meet: u64,
    pub(super) rows_without_state: u64,
    pub(super) schools_minted: u64,
    pub(super) schools_resolved: u64,
    pub(super) fetches_failed: u64,
}

#[derive(Default)]
pub(super) struct Accumulator {
    pub(super) schools: HashMap<String, CanonicalSchool>,
    pub(super) meets: HashMap<String, CanonicalMeet>,
    pub(super) teams: HashMap<String, CanonicalTeam>,
    pub(super) athletes: HashMap<String, CanonicalAthlete>,
    pub(super) events: HashMap<String, CanonicalEvent>,
    pub(super) performances: HashMap<String, CanonicalPerformance>,
    pub(super) unsupported: crate::cohort::UnsupportedCohortRows,
    pub(super) profile_observations: Vec<SourceObservation>,
    pub(super) profile_reviews: Vec<ReviewCase>,
}
pub(super) struct SchoolResolveContext<'a> {
    pub(super) state: Option<UsJurisdiction>,
    pub(super) school_names: &'a HashMap<String, &'a str>,
    pub(super) index: &'a SchoolIndex,
    pub(super) resolved: &'a mut HashMap<String, SchoolId>,
    pub(super) source: &'a SourceRef,
    pub(super) observed_on: &'a str,
    pub(super) stats: &'a mut Stats,
    pub(super) accumulated: &'a mut Accumulator,
}

pub(super) fn grade_in(observed: &[ObservedGrade], school_year: SchoolYear) -> Option<Grade> {
    observed
        .iter()
        .find(|observation| observation.school_year == school_year)
        .map(|observation| observation.grade)
}

pub(super) fn school_for(
    school_id: &str,
    context: &mut SchoolResolveContext<'_>,
) -> Option<SchoolId> {
    let state = context.state?;
    let key = format!("{state}:{school_id}");
    if let Some(id) = context.resolved.get(&key) {
        return Some(id.clone());
    }
    let Some(name) = context.school_names.get(school_id).map(|name| name.to_string()) else {
        context.stats.rows_unknown_school =
            context.stats.rows_unknown_school.saturating_add(1);
        return None;
    };
    if let Some((id, _)) = context.index.resolve(state, &name) {
        context.stats.schools_resolved =
            context.stats.schools_resolved.saturating_add(1);
        context.resolved.insert(key, id.clone());
        return Some(id);
    }
    let (mut school, id) = CanonicalSchool::new(state, name.clone(), name.to_lowercase(), None);
    school.source_identities.push(SourceIdentity {
        namespace: SourceNamespace::AthleticNet {
            kind: SCHOOL_KIND.to_string(),
        },
        id: school_id.to_string(),
        url: None,
    });
    school.evidence.push(Evidence::parsed(
        context.source.clone(),
        context.observed_on,
    ));
    context.stats.schools_minted =
        context.stats.schools_minted.saturating_add(1);
    let id = context
        .accumulated
        .schools
        .entry(id.as_str().to_string())
        .or_insert(school)
        .id
        .clone();
    context.resolved.insert(key, id.clone());
    Some(id)
}

pub(super) fn meet_for(
    meet_id: Option<i64>,
    bio: &Bio,
    state: Option<UsJurisdiction>,
    source: &SourceRef,
    observed_on: &str,
    accumulated: &mut Accumulator,
) -> Option<CanonicalMeet> {
    let state = state?;
    let meet_id = meet_id?;
    let published = bio.meets.get(&meet_id.to_string())?;
    let date = published.date()?.to_string();
    let meet = accumulated
        .meets
        .entry(format!("{state}:{meet_id}"))
        .or_insert_with(|| {
            let mut meet = CanonicalMeet::new(
                Some(state),
                published.name.clone(),
                date.clone(),
                CompetitionLevel::Unknown,
            );
            meet.source_identities.push(SourceIdentity {
                namespace: SourceNamespace::AthleticNet {
                    kind: "meet".to_string(),
                },
                id: meet_id.to_string(),
                url: None,
            });
            meet.evidence
                .push(Evidence::parsed(source.clone(), observed_on));
            meet
        });
    Some(meet.clone())
}

pub(super) struct PerformanceInput<'a> {
    pub(super) athlete: &'a AthleteId,
    pub(super) source_athlete: SourceIdentity,
    pub(super) school: &'a SchoolId,
    pub(super) meet: &'a CanonicalMeet,
    pub(super) kind: &'a EventKind,
    pub(super) sport: Option<Sport>,
    pub(super) gender: Gender,
    pub(super) school_year: SchoolYear,
    pub(super) performance_as_of: chrono::NaiveDate,
    pub(super) grade: Option<Grade>,
    pub(super) date: String,
    pub(super) mark: Mark,
    pub(super) wind_mps: Option<f64>,
    pub(super) place: Option<&'a str>,
    pub(super) round: Option<String>,
    pub(super) timing: Option<TimingMethod>,
    pub(super) division: Option<String>,
    pub(super) source_key: String,
    pub(super) labels: &'a [&'a str],
}

pub(super) fn store_performance(
    accumulated: &mut Accumulator,
    source: &SourceRef,
    observed_on: &str,
    input: PerformanceInput<'_>,
) -> crate::CrawlResult<()> {
    let event = events::ensure_event(accumulated, source, observed_on, &input)?;
    if !admit_date(&input)? {
        return Ok(());
    }
    let team_id = ensure_team(accumulated, source, observed_on, &input);
    let performance_id = CanonicalPerformance::mint(
        input.athlete,
        &input.meet.id,
        &event,
        &input.date,
        &input.source_key,
    );
    accumulated
        .performances
        .entry(performance_id.as_str().to_string())
        .or_insert_with(|| CanonicalPerformance {
            id: performance_id,
            athlete: input.athlete.clone(),
            team: team_id,
            event,
            meet: input.meet.id.clone(),
            date: input.date,
            mark: input.mark,
            wind_mps: input.wind_mps,
            place: input.place.and_then(|place| place.trim().parse().ok()),
            heat: None,
            round: input.round,
            timing: input.timing,
            observed_grade: input.grade,
            evidence: vec![Evidence::parsed(source.clone(), observed_on)],
            source_key: input.source_key,
            source_athlete: Some(input.source_athlete),
            retained_conflicts: Vec::new(),
        });
    Ok(())
}

pub(super) fn ensure_team(
    accumulated: &mut Accumulator,
    source: &SourceRef,
    observed_on: &str,
    input: &PerformanceInput<'_>,
) -> TeamId {
    let sport = match input.sport {
        Some(sport) => sport,
        None => Sport::Unknown,
    };
    let team_id = CanonicalTeam::mint(input.school, sport, input.gender, input.school_year);
    if !accumulated.teams.contains_key(team_id.as_str()) {
        accumulated.teams.insert(
            team_id.as_str().to_string(),
            CanonicalTeam {
                id: team_id.clone(),
                school: input.school.clone(),
                sport,
                gender: input.gender,
                school_year: input.school_year,
                level: Some("high_school".to_string()),
                source_identities: Vec::new(),
                evidence: vec![Evidence::parsed(source.clone(), observed_on)],
                retained_conflicts: Vec::new(),
            },
        );
    }
    team_id
}

pub(super) fn admit_date(input: &PerformanceInput<'_>) -> crate::CrawlResult<bool> {
    use crate::context::{assess_performance_date, PerformanceDateAssessment};
    match assess_performance_date(input.performance_as_of, &input.date) {
        PerformanceDateAssessment::Admitted => Ok(true),
        PerformanceDateAssessment::Future => Ok(false),
        PerformanceDateAssessment::Unknown => Err(crate::CrawlError::PerformanceDateUnknown {
            published: input.date.chars().take(64).collect(),
            as_of: input.performance_as_of,
        }),
    }
}
