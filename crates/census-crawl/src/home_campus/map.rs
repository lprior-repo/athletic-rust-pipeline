use super::parse::{parse_sport_and_gender, CoachRow, FacultyRow};
use super::SOURCE_ID;
use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, SchoolId,
    SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use census_domain::UsJurisdiction;
use serde_json::json;

#[derive(Debug, Clone, Copy)]
pub struct ProfileFacts<'a> {
    pub state: UsJurisdiction,
    pub association: &'a str,
    pub name: &'a str,
    pub city: &'a str,
    pub address: &'a str,
    pub zip: &'a str,
    pub league: &'a str,
    pub phone: &'a str,
    pub url: &'a str,
    pub observed_on: &'a str,
}

#[derive(Debug, Clone)]
pub struct SchoolExtract {
    pub school: CanonicalSchool,
    pub school_id: SchoolId,
    pub state: UsJurisdiction,
    pub association: String,
    pub coaches: Vec<CanonicalCoach>,
}

pub fn school_entities(
    facts: &ProfileFacts<'_>,
    coaches: &[CoachRow],
    faculties: &[FacultyRow],
) -> SchoolExtract {
    let normalized = normalize_name(facts.name);
    let (mut school, school_id) = CanonicalSchool::new(
        facts.state,
        facts.name,
        &normalized,
        if facts.city.is_empty() {
            None
        } else {
            Some(facts.city)
        },
    );
    school.association = Some(facts.association.to_string());

    school.evidence.push(school_evidence(facts));

    let coaches = coach_entities(&school_id, facts, coaches, faculties);

    SchoolExtract {
        school,
        school_id,
        state: facts.state,
        association: facts.association.to_string(),
        coaches,
    }
}

fn school_evidence(facts: &ProfileFacts<'_>) -> Evidence {
    let mut evidence = Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(facts.url.to_string())),
        facts.observed_on.to_string(),
    );
    if !facts.address.is_empty() || !facts.zip.is_empty() || !facts.league.is_empty() {
        evidence.note = Some(
            json!({
                "published_address": facts.address,
                "zip": facts.zip,
                "league": facts.league,
                "phone": facts.phone,
            })
            .to_string(),
        );
    }
    evidence
}

fn coach_entities(
    school_id: &SchoolId,
    facts: &ProfileFacts<'_>,
    coaches: &[CoachRow],
    faculties: &[FacultyRow],
) -> Vec<CanonicalCoach> {
    let mut entities: Vec<CanonicalCoach> = Vec::new();

    for row in coaches {
        if row.name.is_empty() {
            continue;
        }
        let Some((sport, gender)) = parse_sport_and_gender(&row.sport) else {
            continue;
        };
        if let Some(coach) = sport_coach(
            school_id,
            row,
            sport,
            gender,
            facts.association,
            facts.url,
            facts.observed_on,
        ) {
            entities.push(coach);
        }
    }

    for row in faculties {
        if row.role != "Athletic Director" || row.name.is_empty() {
            continue;
        }
        if let Some(coach) = director(
            school_id,
            row,
            facts.association,
            facts.url,
            facts.observed_on,
        ) {
            entities.push(coach);
        }
    }

    entities
}

fn sport_coach(
    school_id: &SchoolId,
    row: &CoachRow,
    sport: Sport,
    gender: Gender,
    association: &str,
    profile_url: &str,
    observed_on: &str,
) -> Option<CanonicalCoach> {
    let mut coach = CanonicalCoach::new(
        school_id,
        row.name.clone(),
        Some(sport),
        gender,
        role_of(&row.role),
    );

    if !row.email.is_empty() {
        coach.set_published_email(&row.email);
    }

    coach.source_identities.push(
        SourceIdentity::new(
            SourceNamespace::AssociationSchool {
                association: association.to_string(),
            },
            format!("coach:{}:{}", row.user_id, row.sport_id),
        )
        .with_url(profile_url.to_string()),
    );

    coach.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(profile_url.to_string())),
        observed_on.to_string(),
    ));

    Some(coach)
}

fn director(
    school_id: &SchoolId,
    row: &FacultyRow,
    association: &str,
    profile_url: &str,
    observed_on: &str,
) -> Option<CanonicalCoach> {
    let mut coach = CanonicalCoach::new(
        school_id,
        row.name.clone(),
        None,
        Gender::Unknown,
        CoachRole::AthleticDirector,
    );

    if !row.email.is_empty() {
        coach.set_published_email(&row.email);
    }

    coach.source_identities.push(
        SourceIdentity::new(
            SourceNamespace::AssociationSchool {
                association: association.to_string(),
            },
            format!("ad:{}", row.id),
        )
        .with_url(profile_url.to_string()),
    );

    coach.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(profile_url.to_string())),
        observed_on.to_string(),
    ));

    Some(coach)
}

fn role_of(value: &str) -> CoachRole {
    match value.trim() {
        "Head Coach" => CoachRole::HeadCoach,
        "Assistant Coach" => CoachRole::AssistantCoach,
        _ => CoachRole::Unknown,
    }
}
