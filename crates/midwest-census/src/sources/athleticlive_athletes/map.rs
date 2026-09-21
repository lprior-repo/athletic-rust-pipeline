//! Canonical mapping: the athlete-row entity pass and the Athletic.net profile seeds it
//! mints.

use super::parse::{AthleteHit, HitTeam};
use super::targets::MeetTarget;
use super::tokens::{gender_from_token, grade_from_token, school_year_for_date, sport_for};
use super::ENDPOINT;
use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, CanonicalTeam, Evidence, Gender, GradYear, Grade,
    ObservedGrade, SchoolId, SchoolYear, SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

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
    let mut minted = Minted::default();

    for hit in hits {
        out.rows += 1;
        absorb_hit(
            hit,
            targets,
            observed_on,
            fallback_year,
            &mut minted,
            &mut out,
        );
    }

    out.schools = minted.schools.into_values().collect();
    out.teams = minted.teams.into_values().collect();
    out.athletes = minted.athletes.into_values().collect();
    out
}

/// The canonical entities one batch accumulates, keyed by canonical id so re-publications merge.
#[derive(Default)]
struct Minted {
    schools: BTreeMap<String, CanonicalSchool>,
    teams: BTreeMap<(String, String, String, SchoolYear), CanonicalTeam>,
    athletes: BTreeMap<String, CanonicalAthlete>,
}

/// One row's decoded facts: the meet target it belongs to, its name, school and evidence.
struct RowFacts<'a> {
    name: &'a str,
    school_name: &'a str,
    target: &'a MeetTarget,
    team: &'a HitTeam,
    source: SourceRef,
    evidence: Evidence,
    gender: Gender,
    grade: Option<Grade>,
    sport: Sport,
    school_year: SchoolYear,
}

/// Decode one row, or `None` when it carries no usable meet, name or school.
///
/// Only a row whose team names no school counts against `rows_without_school`; a row missing its
/// meet, name or grade is simply not an entity.
fn decode_row<'a>(
    hit: &'a AthleteHit,
    targets: &'a HashMap<u64, &'a MeetTarget>,
    observed_on: &str,
    fallback_year: SchoolYear,
    out: &mut BatchEntities,
) -> Option<RowFacts<'a>> {
    let meet_id = hit.meet_id()?;
    let target = targets.get(&meet_id)?;
    let name = hit.n.as_deref().map(str::trim).filter(|n| !n.is_empty())?;
    let Some(team) = hit.t.as_ref().filter(|t| t.school_name().is_some()) else {
        out.rows_without_school += 1;
        return None;
    };
    let source = SourceRef::new(
        "athleticlive_athletes",
        Some(format!("{ENDPOINT} (mi={meet_id})")),
    );
    let evidence = Evidence::parsed(source.clone(), observed_on);
    let gender = hit
        .g
        .as_deref()
        .map(gender_from_token)
        .unwrap_or(Gender::Unknown);
    Some(RowFacts {
        name,
        school_name: team.school_name().unwrap_or_default().trim(),
        target,
        team,
        source,
        evidence,
        gender,
        grade: hit.y.as_ref().and_then(|value| match value {
            Value::String(s) => grade_from_token(s),
            Value::Number(n) => grade_from_token(&n.to_string()),
            _ => None,
        }),
        sport: sport_for(team.is_cross_country(), &target.name, &target.date),
        school_year: school_year_for_date(&target.date, fallback_year),
    })
}

/// Fold one athlete row into the batch's schools, teams and athletes.
fn absorb_hit(
    hit: &AthleteHit,
    targets: &HashMap<u64, &MeetTarget>,
    observed_on: &str,
    fallback_year: SchoolYear,
    minted: &mut Minted,
    out: &mut BatchEntities,
) {
    let Some(row) = decode_row(hit, targets, observed_on, fallback_year, out) else {
        return;
    };
    // School: canonical id from state + normalized name, so a MileSplit school of the same name
    // and state mints the same school.
    let school_id = row_school(&mut minted.schools, &row);
    let team_entry = row_team(&mut minted.teams, &school_id, &row);
    note_team_ids(row.team, row.target, team_entry, &mut out.rows_with_team_id);
    absorb_athlete(hit, &row, &school_id, minted, out);
}

/// Mint or reuse the athlete one row names, adding that row's grade observation to it.
fn absorb_athlete(
    hit: &AthleteHit,
    row: &RowFacts<'_>,
    school_id: &SchoolId,
    minted: &mut Minted,
    out: &mut BatchEntities,
) {
    // Grade is required: an athlete entity is minted from (school, name, grad year, gender).
    let Some(grade) = row.grade else { return };
    out.rows_with_grade += 1;
    let grad_year = GradYear::of(grade, row.school_year);
    let athlete_id = CanonicalAthlete::mint(school_id, row.name, grad_year, row.gender);
    let entry = minted
        .athletes
        .entry(athlete_id.as_str().to_string())
        .or_insert_with(|| {
            let mut athlete = CanonicalAthlete::new(school_id, row.name, grad_year, row.gender);
            athlete.sports.push(row.sport);
            athlete.evidence.push(row.evidence.clone());
            athlete
        });
    if !entry.sports.contains(&row.sport) {
        entry.sports.push(row.sport);
    }
    let observation = ObservedGrade {
        grade,
        school_year: row.school_year,
        source: row.source.clone(),
    };
    if !entry.observed_grades.contains(&observation) {
        entry.observed_grades.push(observation);
    }
    note_athlete_ids(entry, hit, &mut out.rows_with_athlete_id);
}

/// The canonical school one row's competitor belongs to, minted or reused by state + name.
fn row_school(schools: &mut BTreeMap<String, CanonicalSchool>, row: &RowFacts<'_>) -> SchoolId {
    let (mut school, school_id) = CanonicalSchool::new(
        &row.target.state,
        row.school_name,
        census_domain::model::normalize_name(row.school_name),
    );
    if !school.evidence.iter().any(|e| e == &row.evidence) {
        school.evidence.push(row.evidence.clone());
    }
    schools
        .entry(school_id.as_str().to_string())
        .or_insert(school);
    school_id
}

/// The team one row's competitor belongs to: one per (school, sport, gender, school year).
///
/// The key is the same one MileSplit uses, so both sources mint the same team id for one team.
fn row_team<'a>(
    teams: &'a mut BTreeMap<(String, String, String, SchoolYear), CanonicalTeam>,
    school_id: &SchoolId,
    row: &RowFacts<'_>,
) -> &'a mut CanonicalTeam {
    let team_key = (
        school_id.as_str().to_string(),
        format!("{:?}", row.sport),
        format!("{:?}", row.gender),
        row.school_year,
    );
    teams.entry(team_key).or_insert_with(|| {
        let id = CanonicalTeam::mint(school_id, row.sport, row.gender, row.school_year);
        CanonicalTeam {
            id,
            school: school_id.clone(),
            sport: row.sport,
            gender: row.gender,
            school_year: row.school_year,
            level: Some("high_school".to_string()),
            source_identities: Vec::new(),
            evidence: vec![row.evidence.clone()],
        }
    })
}

/// Record the timer and Athletic.net team ids one row publishes on its team.
fn note_team_ids(
    team: &HitTeam,
    target: &MeetTarget,
    team_entry: &mut CanonicalTeam,
    rows_with_team_id: &mut usize,
) {
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
        *rows_with_team_id += 1;
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
}

/// Record the Athletic.net athlete id and profile URL one row publishes on its athlete.
fn note_athlete_ids(
    entry: &mut CanonicalAthlete,
    hit: &AthleteHit,
    rows_with_athlete_id: &mut usize,
) {
    if let Some(an_athlete_id) = hit.athletic_net_athlete_id() {
        *rows_with_athlete_id += 1;
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
