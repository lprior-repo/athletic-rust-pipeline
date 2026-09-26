//! Roster -> canonical entities: the school, its athletes with their observed grade, and one
//! team per sport the roster carries.
use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalSchool, CanonicalTeam, Evidence, Gender,
    GradYear, Grade, ObservedGrade, SchoolId, SchoolYear, SourceIdentity, SourceNamespace,
    SourceRef, Sport,
};

use super::wire::{Roster, RosterAthlete, Site, TeamRef};

impl RosterAthlete {
    pub fn sports(&self) -> Vec<Sport> {
        let mut sports = Vec::new();
        if self.indoor {
            sports.push(Sport::IndoorTrack);
        }
        if self.outdoor {
            sports.push(Sport::OutdoorTrack);
        }
        if self.xc {
            sports.push(Sport::CrossCountry);
        }
        sports
    }

    /// The school year this roster was observed in. Rosters are current-season documents; the
    /// caller passes the school year the collection belongs to.
    pub fn observed_grade(
        &self,
        school_year: SchoolYear,
        source: SourceRef,
    ) -> Option<ObservedGrade> {
        // 13 - (grad_year - school_year_start): both steps are checked so an out-of-range year
        // pair can only yield `None`, never a wrapped or panicking grade.
        let years_to_graduation = self.grad_year.get().checked_sub(school_year.get())?;
        let grade_number = 13_i16.checked_sub(years_to_graduation)?;
        Grade::new(u8::try_from(grade_number).ok()?).map(|grade| ObservedGrade {
            grade,
            school_year,
            source,
        })
    }
}

/// Convert a parsed roster into canonical entities.
///
/// The state every entity is filed under comes from the [`Site`] the roster was fetched through, so
/// there is no second, string-shaped copy of the jurisdiction for a caller to get wrong.
pub fn roster_entities(
    roster: &Roster,
    school_year: SchoolYear,
    observed_on: &str,
    site: &Site,
) -> (CanonicalSchool, Vec<CanonicalAthlete>, Vec<CanonicalTeam>) {
    let (school, school_id, source) = roster_school(roster, observed_on, site);

    let mut seen_sports: Vec<(Sport, Gender)> = Vec::new();
    let mut athletes = Vec::new();
    for entry in &roster.athletes {
        for sport in entry.sports() {
            let key = (sport, entry.gender);
            if !seen_sports.contains(&key) {
                seen_sports.push(key);
            }
        }
        athletes.push(roster_athlete_entity(
            entry,
            &school_id,
            &source,
            school_year,
            observed_on,
            site,
        ));
    }

    let teams = roster_teams(
        seen_sports,
        &school_id,
        school_year,
        &source,
        &roster.team,
        observed_on,
    );
    (school, athletes, teams)
}

/// The school a roster belongs to, plus the id it minted and the source reference every entity of
/// the roster is stamped with.
fn roster_school(
    roster: &Roster,
    observed_on: &str,
    site: &Site,
) -> (CanonicalSchool, SchoolId, SourceRef) {
    let source = SourceRef::new(
        site.source_id(),
        Some(format!("{}/roster", roster.team.url)),
    );
    let owner = owner_name(&roster.team);
    let (mut school, school_id) =
        CanonicalSchool::new(site.jurisdiction(), &owner, normalize_name(&owner));
    school.city = city_of(&roster.team.city_state);
    school.source_identities.push(
        SourceIdentity::new(SourceNamespace::MilesplitSchool, roster.team.id.clone())
            .with_url(roster.team.url.clone()),
    );
    school
        .evidence
        .push(Evidence::parsed(source.clone(), observed_on.to_string()));
    (school, school_id, source)
}

/// One roster entry as a canonical athlete: its names, sports, observed grade, profile identity and
/// the evidence that ties it back to the page it was read from.
fn roster_athlete_entity(
    entry: &RosterAthlete,
    school_id: &SchoolId,
    source: &SourceRef,
    school_year: SchoolYear,
    observed_on: &str,
    site: &Site,
) -> CanonicalAthlete {
    let mut athlete = CanonicalAthlete::new(school_id, entry.name.clone(), entry.grad_year, entry.gender,
    SourceIdentity::new(SourceNamespace::MilesplitAthlete, entry.athlete_id.clone())
        .with_url(entry.profile_url.clone()),);
    athlete.known_names = vec![entry.name.clone(), entry.roster_name.clone()];
    athlete.sports = entry.sports();
    if let Some(observation) = entry.observed_grade(school_year, source.clone()) {
        athlete.observed_grades.push(observation);
    }
    athlete.public_profile_urls.push(entry.profile_url.clone());
    athlete.evidence.push(Evidence::parsed(
        SourceRef::new(site.source_id(), Some(entry.profile_url.clone())),
        observed_on.to_string(),
    ));
    athlete
}

/// One canonical team per sport/gender the roster carries, keyed by the team page's own id.
fn roster_teams(
    seen_sports: Vec<(Sport, Gender)>,
    school_id: &SchoolId,
    school_year: SchoolYear,
    source: &SourceRef,
    team: &TeamRef,
    observed_on: &str,
) -> Vec<CanonicalTeam> {
    seen_sports
        .into_iter()
        .map(|(sport, gender)| {
            let id = CanonicalTeam::mint(school_id, sport, gender, school_year);
            CanonicalTeam {
                id,
                school: school_id.clone(),
                sport,
                gender,
                school_year,
                level: Some("high_school".to_string()),
                source_identities: vec![SourceIdentity::new(
                    SourceNamespace::MilesplitTeam,
                    team.id.clone(),
                )
                .with_url(team.url.clone())],
                evidence: vec![Evidence::parsed(source.clone(), observed_on.to_string())],
                retained_conflicts: Vec::new(),
            }
        })
        .collect()
}

/// MileSplit team rows use the school name; strip a trailing gender marker if present.
fn owner_name(team: &TeamRef) -> String {
    let name = team.name.trim();
    for suffix in [" Boys", " Girls", " (B)", " (G)"] {
        if let Some(stripped) = name.strip_suffix(suffix) {
            return stripped.trim().to_string();
        }
    }
    name.to_string()
}

fn city_of(city_state: &str) -> Option<String> {
    let first = city_state.split(',').next()?.trim();
    if first.is_empty() {
        None
    } else {
        Some(title_case(first))
    }
}

fn title_case(value: &str) -> String {
    value
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str().to_lowercase()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
