//! Canonical mapping: the meet targets the adapter will query, the athlete-row entity
//! pass and the Athletic.net profile seeds it mints.

use super::parse::AthleteHit;
use super::tokens::{gender_from_token, grade_from_token, school_year_for_date, sport_for};
use super::ENDPOINT;
use crate::model::{
    CanonicalAthlete, CanonicalMeet, CanonicalSchool, CanonicalTeam, Evidence, Gender, GradYear,
    ObservedGrade, SchoolYear, SourceIdentity, SourceNamespace, SourceRef,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// A canonical meet plus the timer identity this adapter queries it by.
#[derive(Debug, Clone)]
pub struct MeetTarget {
    pub athleticlive_meet_id: u64,
    pub meet_id: String,
    pub tenant: String,
    pub name: String,
    pub state: String,
    pub date: String,
}

/// Meets the adapter will query, plus the count it refused to query.
#[derive(Debug, Clone, Default)]
pub struct MeetSelection {
    pub targets: Vec<MeetTarget>,
    /// Timer-published meets dropped because their published date cannot be trusted.
    ///
    /// Tenant meet indexes carry placeholder rows (`date` in the 2220s). An athlete's graduating
    /// class is derived from the meet date, so an implausible date would mint an implausible
    /// class; those meets are skipped rather than guessed at.
    pub skipped_implausible: usize,
}

/// The window a meet date must fall in to be usable. Same convention as the meet-index harvest.
const MEET_YEAR_MIN: i16 = 2015;
const MEET_YEAR_MAX: i16 = 2030;

fn plausible_meet_year(date: &str) -> bool {
    match date.get(..4).and_then(|year| year.parse::<i16>().ok()) {
        Some(year) => (MEET_YEAR_MIN..=MEET_YEAR_MAX).contains(&year),
        None => false,
    }
}

/// Select meets that a timer published, keyed by their AthleticLIVE meet id.
///
/// Meets are deduplicated by canonical id: several tenants publishing one meet collapse to the first
/// target, because the athlete rows are keyed by the AthleticLIVE meet id and duplicate ids would
/// multiply requests.
pub fn meet_targets(meets: &[CanonicalMeet], states: &[String]) -> MeetSelection {
    let wanted: BTreeSet<String> = states
        .iter()
        .map(|state| state.trim().to_ascii_uppercase())
        .collect();
    let mut seen: HashMap<u64, MeetTarget> = HashMap::new();
    let mut skipped_implausible = 0usize;
    for meet in meets {
        if !wanted.is_empty() && !wanted.contains(&meet.state.to_ascii_uppercase()) {
            continue;
        }
        if !plausible_meet_year(&meet.date) {
            skipped_implausible += 1;
            continue;
        }
        for identity in &meet.source_identities {
            let SourceNamespace::TimerMeet { provider } = &identity.namespace else {
                continue;
            };
            let Ok(athleticlive_meet_id) = identity.id.parse::<u64>() else {
                continue;
            };
            seen.entry(athleticlive_meet_id)
                .or_insert_with(|| MeetTarget {
                    athleticlive_meet_id,
                    meet_id: meet.id.as_str().to_string(),
                    tenant: provider.clone(),
                    name: meet.name.clone(),
                    state: meet.state.clone(),
                    date: meet.date.clone(),
                });
        }
    }
    let mut targets: Vec<MeetTarget> = seen.into_values().collect();
    targets.sort_by_key(|target| {
        (
            target.state.clone(),
            target.date.clone(),
            target.athleticlive_meet_id,
        )
    });
    MeetSelection {
        targets,
        skipped_implausible,
    }
}

/// Entities minted from one batch of athlete rows.
#[derive(Debug, Default)]
pub struct BatchEntities {
    pub schools: Vec<CanonicalSchool>,
    pub teams: Vec<CanonicalTeam>,
    pub athletes: Vec<CanonicalAthlete>,
    pub rows: usize,
    pub rows_with_grade: usize,
    pub rows_with_athlete_id: usize,
    pub rows_with_team_id: usize,
    pub rows_without_school: usize,
}

/// Canonical entities for a batch of athlete rows, deduplicated by canonical id.
///
/// The athlete key is (school, name, grad year, gender): two meets that disagree on nothing produce
/// one athlete, and the second meet's grade observation is appended rather than replacing the first.
pub fn build_entities(
    hits: &[AthleteHit],
    targets: &HashMap<u64, &MeetTarget>,
    observed_on: &str,
    fallback_year: SchoolYear,
) -> BatchEntities {
    let mut out = BatchEntities::default();
    let mut schools: BTreeMap<String, CanonicalSchool> = BTreeMap::new();
    let mut teams: BTreeMap<(String, String, String, SchoolYear), CanonicalTeam> = BTreeMap::new();
    let mut athletes: BTreeMap<String, CanonicalAthlete> = BTreeMap::new();

    for hit in hits {
        out.rows += 1;
        let Some(meet_id) = hit.meet_id() else {
            continue;
        };
        let Some(target) = targets.get(&meet_id) else {
            continue;
        };
        let Some(name) = hit.n.as_deref().map(str::trim).filter(|n| !n.is_empty()) else {
            continue;
        };
        let Some(team) = hit.t.as_ref().filter(|t| t.school_name().is_some()) else {
            out.rows_without_school += 1;
            continue;
        };
        let school_name = team.school_name().unwrap_or_default().trim();
        let source_url = format!("{ENDPOINT} (mi={meet_id})");
        let evidence = Evidence::parsed(
            SourceRef::new("athleticlive_athletes", Some(source_url.clone())),
            observed_on,
        );

        let gender = hit
            .g
            .as_deref()
            .map(gender_from_token)
            .unwrap_or(Gender::Unknown);
        let grade = hit.y.as_ref().and_then(|value| match value {
            Value::String(s) => grade_from_token(s),
            Value::Number(n) => grade_from_token(&n.to_string()),
            _ => None,
        });
        let sport = sport_for(team.is_cross_country(), &target.name, &target.date);
        let school_year = school_year_for_date(&target.date, fallback_year);

        // School: canonical id from state + normalized name, so a MileSplit school of the same name
        // and state mints the same school.
        let (mut school, school_id) = CanonicalSchool::new(
            &target.state,
            school_name,
            crate::model::normalize_name(school_name),
        );
        if !school.evidence.iter().any(|e| e == &evidence) {
            school.evidence.push(evidence.clone());
        }
        schools
            .entry(school_id.as_str().to_string())
            .or_insert(school);

        // Team: one per (school, sport, gender, school year).
        // One team per (school, sport, gender, school year) - the same key MileSplit uses, so
        // both sources mint the same team id for the same team.
        let team_key = (
            school_id.as_str().to_string(),
            format!("{sport:?}"),
            format!("{gender:?}"),
            school_year,
        );
        let team_entry = teams.entry(team_key).or_insert_with(|| {
            let id = CanonicalTeam::mint(&school_id, sport, gender, school_year);
            CanonicalTeam {
                id,
                school: school_id.clone(),
                sport,
                gender,
                school_year,
                level: Some("high_school".to_string()),
                source_identities: Vec::new(),
                evidence: vec![evidence.clone()],
            }
        });
        if let Some(timer_team_id) = team.athleticlive_team_id() {
            let identity = SourceIdentity::new(
                SourceNamespace::TimerTeam {
                    provider: target.tenant.clone(),
                },
                timer_team_id.to_string(),
            );
            if !team_entry.source_identities.contains(&identity) {
                team_entry.source_identities.push(identity);
            }
        }
        if let Some(an_team_id) = team.athletic_net_team_id() {
            out.rows_with_team_id += 1;
            let identity = SourceIdentity::new(
                SourceNamespace::LegacyAthleticNet {
                    kind: "team".to_string(),
                },
                an_team_id.to_string(),
            );
            if !team_entry.source_identities.contains(&identity) {
                team_entry.source_identities.push(identity);
            }
        }

        // Grade is required: an athlete entity is minted from (school, name, grad year, gender).
        let Some(grade) = grade else { continue };
        out.rows_with_grade += 1;
        let grad_year = GradYear::of(grade, school_year);
        let athlete_id = CanonicalAthlete::mint(&school_id, name, grad_year, gender);
        let entry = athletes
            .entry(athlete_id.as_str().to_string())
            .or_insert_with(|| {
                let mut athlete = CanonicalAthlete::new(&school_id, name, grad_year, gender);
                athlete.sports.push(sport);
                athlete.evidence.push(evidence.clone());
                athlete
            });
        if !entry.sports.contains(&sport) {
            entry.sports.push(sport);
        }
        let observation = ObservedGrade {
            grade,
            school_year,
            source: SourceRef::new("athleticlive_athletes", Some(source_url.clone())),
        };
        if !entry.observed_grades.contains(&observation) {
            entry.observed_grades.push(observation);
        }
        if let Some(an_athlete_id) = hit.athletic_net_athlete_id() {
            out.rows_with_athlete_id += 1;
            let identity = SourceIdentity::new(
                SourceNamespace::LegacyAthleticNet {
                    kind: "athlete".to_string(),
                },
                an_athlete_id.to_string(),
            );
            if !entry.source_identities.contains(&identity) {
                entry.source_identities.push(identity);
            }
            // Athletic.net profile URLs are deterministic from the athlete id (research report 02).
            let profile_url =
                format!("https://www.athletic.net/athlete/{an_athlete_id}/track-and-field");
            if !entry.public_profile_urls.contains(&profile_url) {
                entry.public_profile_urls.push(profile_url);
            }
        }
    }

    out.schools = schools.into_values().collect();
    out.teams = teams.into_values().collect();
    out.athletes = athletes.into_values().collect();
    out
}
