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
    pub rows_without_subject_id: usize,
}

pub fn build_entities(
    hits: &[AthleteHit],
    targets: &HashMap<u64, &MeetTarget>,
    observed_on: &str,
    fallback_year: SchoolYear,
) -> BatchEntities {
    let mut out = BatchEntities::default();
    let mut minted = Minted::default();

    for hit in hits {
        out.rows = out.rows.saturating_add(1);
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

#[derive(Default)]
struct Minted {
    schools: BTreeMap<String, CanonicalSchool>,
    teams: BTreeMap<(String, String, String, SchoolYear), CanonicalTeam>,
    athletes: BTreeMap<String, CanonicalAthlete>,
}

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
        out.rows_without_school = out.rows_without_school.saturating_add(1);
        return None;
    };
    let source = SourceRef::new("athleticlive_athletes", Some(ENDPOINT.to_owned()));
    let evidence = Evidence::parsed(source.clone(), observed_on);
    let gender = hit
        .g
        .as_deref()
        .map(gender_from_token)
        .map_or(Gender::Unknown, |value| value);
    Some(RowFacts {
        name,
        school_name: team
            .school_name()
            .map_or(Default::default(), core::convert::identity)
            .trim(),
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
    let school_id = row_school(&mut minted.schools, &row);
    let team_entry = row_team(&mut minted.teams, &school_id, &row);
    note_team_ids(row.team, row.target, team_entry, &mut out.rows_with_team_id);
    absorb_athlete(hit, &row, &school_id, minted, out);
}

fn absorb_athlete(
    hit: &AthleteHit,
    row: &RowFacts<'_>,
    school_id: &SchoolId,
    minted: &mut Minted,
    out: &mut BatchEntities,
) {
    let Some(grade) = row.grade else { return };
    out.rows_with_grade = out.rows_with_grade.saturating_add(1);
    let Some(grad_year) = GradYear::of(grade, row.school_year) else {
        return;
    };
    let Some(source) = source_identity(hit, &row.target.tenant, row.source.url.as_deref()) else {
        out.rows_without_subject_id = out.rows_without_subject_id.saturating_add(1);
        return;
    };
    let athlete_id = CanonicalAthlete::mint(school_id, row.name, grad_year, row.gender, &source);
    let entry = minted
        .athletes
        .entry(athlete_id.as_str().to_string())
        .or_insert_with(|| {
            let mut athlete =
                CanonicalAthlete::new(school_id, row.name, grad_year, row.gender, source);
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

fn row_school(schools: &mut BTreeMap<String, CanonicalSchool>, row: &RowFacts<'_>) -> SchoolId {
    let (mut school, school_id) = CanonicalSchool::new(
        row.target.state,
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
            retained_conflicts: Vec::new(),
        }
    })
}

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
        *rows_with_team_id = (*rows_with_team_id).saturating_add(1);
        let identity = SourceIdentity::new(
            SourceNamespace::athletic_net("team"),
            an_team_id.to_string(),
        );
        if !team_entry.source_identities.contains(&identity) {
            team_entry.source_identities.push(identity);
        }
    }
}

fn source_identity(
    hit: &AthleteHit,
    provider: &str,
    capture_url: Option<&str>,
) -> Option<SourceIdentity> {
    let mut identity = match hit.athletic_net_athlete_id() {
        Some(id) => SourceIdentity::new(SourceNamespace::athletic_net("athlete"), id.to_string()),
        None => {
            let (meet, row) = hit.meet_id().zip(hit.athleticlive_row_id())?;
            SourceIdentity::new(
                SourceNamespace::Other("athleticlive_roster_entry".to_owned()),
                format!("{provider}:meet:{meet}:entry:{row}"),
            )
        }
    };
    identity.url = capture_url.map(str::to_owned);
    Some(identity)
}

fn note_athlete_ids(
    entry: &mut CanonicalAthlete,
    hit: &AthleteHit,
    rows_with_athlete_id: &mut usize,
) {
    if let Some(id) = hit.athletic_net_athlete_id() {
        *rows_with_athlete_id = rows_with_athlete_id.saturating_add(1);
        let profile = format!("https://www.athletic.net/athlete/{id}/track-and-field");
        if !entry.public_profile_urls.contains(&profile) {
            entry.public_profile_urls.push(profile);
        }
    }
}

#[cfg(test)]
#[path = "map_tests.rs"]
mod tests;
