use super::parse::{parse_gender, parse_sport_label, CoachRow};
use super::{ASSOCIATION, SOURCE_ID, STATE};
use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, SchoolId,
    SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use serde_json::json;

#[derive(Debug, Clone, Copy)]
pub struct ProfileFacts<'a> {
    pub name: &'a str,
    pub address: &'a str,
    pub district: &'a str,
    pub classification: &'a str,
    pub region: &'a str,
    pub url: &'a str,
    pub observed_on: &'a str,
}

#[derive(Debug, Clone)]
pub struct SchoolExtract {
    pub school: CanonicalSchool,
    pub school_id: SchoolId,
    pub coaches: Vec<CanonicalCoach>,
}

pub fn school_entities(facts: &ProfileFacts<'_>, coaches: &[CoachRow]) -> SchoolExtract {
    let normalized = normalize_name(facts.name);
    let (mut school, school_id) = CanonicalSchool::new(STATE, facts.name, &normalized, None);

    if !facts.classification.is_empty() {
        school.classification = Some(facts.classification.to_string());
    }

    school.association = Some(ASSOCIATION.to_string());
    let mut evidence = Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(facts.url.to_string())),
        facts.observed_on.to_string(),
    );
    if !facts.address.is_empty() || !facts.district.is_empty() || !facts.region.is_empty() {
        evidence.note = Some(
            json!({
                "published_address": facts.address,
                "district": facts.district,
                "region": facts.region,
            })
            .to_string(),
        );
    }
    school.evidence.push(evidence);

    let mut coach_entities: Vec<CanonicalCoach> = Vec::new();

    for row in coaches {
        if let Some(sport) = parse_sport_label(&row.sport_label) {
            let gender = parse_gender(&row.sport_label);
            if let Some(coach) =
                sport_coach(&school_id, row, sport, gender, facts.url, facts.observed_on)
            {
                coach_entities.push(coach);
            }
        }
    }

    SchoolExtract {
        school,
        school_id,
        coaches: coach_entities,
    }
}

fn sport_coach(
    school_id: &SchoolId,
    row: &CoachRow,
    sport: Sport,
    gender: Gender,
    profile_url: &str,
    observed_on: &str,
) -> Option<CanonicalCoach> {
    let mut coach = CanonicalCoach::new(
        school_id,
        row.name.clone(),
        Some(sport),
        gender,
        CoachRole::HeadCoach,
    );

    if !row.email.is_empty() {
        coach.set_published_email(&row.email);
    }

    coach.source_identities.push(
        SourceIdentity::new(
            SourceNamespace::AssociationSchool {
                association: ASSOCIATION.to_string(),
            },
            format!("coach:{}", row.sport_label),
        )
        .with_url(profile_url.to_string()),
    );

    coach.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(profile_url.to_string())),
        observed_on.to_string(),
    ));

    Some(coach)
}
