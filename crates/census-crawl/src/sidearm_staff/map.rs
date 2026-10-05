use super::parse::{StaffDirectory, StaffRow};
use super::SOURCE_ID;
use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, SchoolId,
    SourceIdentity, SourceNamespace, SourceRef,
};
use census_domain::UsJurisdiction;
use serde_json::json;

#[derive(Debug, Clone, Copy)]
pub struct ProfileFacts<'a> {
    pub state: UsJurisdiction,
    pub host: &'a str,
    pub url: &'a str,
    pub observed_on: &'a str,
}

#[derive(Debug, Clone)]
pub struct SchoolExtract {
    pub school: CanonicalSchool,
    pub school_id: SchoolId,
    pub state: UsJurisdiction,
    pub coaches: Vec<CanonicalCoach>,
}

pub fn namespace(state: UsJurisdiction) -> SourceNamespace {
    SourceNamespace::SchoolDirectory {
        provider: SOURCE_ID.to_string(),
        state,
    }
}

pub fn school_entities(facts: &ProfileFacts<'_>, directory: &StaffDirectory) -> SchoolExtract {
    let (mut school, school_id) = CanonicalSchool::new(
        facts.state,
        &directory.name,
        normalize_name(&directory.name),
    );
    school.athletics_website = Some(facts.host.to_string());
    school
        .source_identities
        .push(SourceIdentity::new(namespace(facts.state), facts.host).with_url(facts.url));
    school.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(facts.url.to_string())),
        facts.observed_on,
    ));
    let coaches = directory
        .members
        .iter()
        .filter_map(|row| coach_entity(&school_id, row, facts))
        .collect();
    SchoolExtract {
        school,
        school_id,
        state: facts.state,
        coaches,
    }
}

fn coach_entity(
    school_id: &SchoolId,
    row: &StaffRow,
    facts: &ProfileFacts<'_>,
) -> Option<CanonicalCoach> {
    if row.name.is_empty() || row.email.is_empty() {
        return None;
    }
    let (sport, gender, role) = if row.role == "Athletic Director" {
        (None, Gender::Unknown, CoachRole::AthleticDirector)
    } else {
        if !row.role.to_ascii_lowercase().contains("coach") {
            return None;
        }
        let (sport, gender) = crate::home_campus::parse_sport_and_gender(&row.sport)?;
        (Some(sport), gender, role_of(&row.role))
    };
    let mut coach = CanonicalCoach::new(school_id, &row.name, sport, gender, role);
    coach.set_published_email(&row.email);
    coach.source_identities.push(
        SourceIdentity::new(
            namespace(facts.state),
            format!("{}:member:{}", facts.host, row.id),
        )
        .with_url(facts.url),
    );
    let mut evidence = Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(facts.url.to_string())),
        facts.observed_on,
    );
    evidence.note = Some(
        json!({
            "member_id": row.id,
            "category_id": row.category_id,
            "published_sport": row.sport,
            "published_level": row.level,
            "published_role": row.role,
        })
        .to_string(),
    );
    coach.evidence.push(evidence);
    Some(coach)
}

fn role_of(value: &str) -> CoachRole {
    match value {
        "Head Coach" => CoachRole::HeadCoach,
        "Assistant Coach" => CoachRole::AssistantCoach,
        _ => CoachRole::Unknown,
    }
}
