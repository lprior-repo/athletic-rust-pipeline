use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalSchool, CanonicalTeam, Evidence, Gender,
    PublishedGraduation, SchoolId, SchoolYear, SourceIdentity, SourceNamespace, SourceRef, Sport,
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
}

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
    let (mut school, school_id) = CanonicalSchool::new(
        site.jurisdiction(),
        &owner,
        normalize_name(&owner),
        city_of(&roster.team.city_state).as_deref(),
    );
    school.source_identities.push(
        SourceIdentity::new(SourceNamespace::MilesplitSchool, roster.team.id.clone())
            .with_url(roster.team.url.clone()),
    );
    school
        .evidence
        .push(Evidence::parsed(source.clone(), observed_on.to_string()));
    (school, school_id, source)
}

fn roster_athlete_entity(
    entry: &RosterAthlete,
    school_id: &SchoolId,
    source: &SourceRef,
    observed_on: &str,
    site: &Site,
) -> CanonicalAthlete {
    let mut athlete = CanonicalAthlete::new(
        school_id,
        entry.name.clone(),
        entry.grad_year,
        entry.gender,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, entry.athlete_id.clone())
            .with_url(entry.profile_url.clone()),
    );
    athlete.known_names = vec![entry.name.clone(), entry.roster_name.clone()];
    athlete.sports = entry.sports();
    athlete.published_graduations.push(PublishedGraduation {
        grad_year: entry.grad_year,
        source: source.clone(),
    });
    athlete.public_profile_urls.push(entry.profile_url.clone());
    athlete.evidence.push(Evidence::parsed(
        SourceRef::new(site.source_id(), Some(entry.profile_url.clone())),
        observed_on.to_string(),
    ));
    athlete
}

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
