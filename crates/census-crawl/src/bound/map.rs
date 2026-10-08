use super::parse::CoachRow;
use super::SOURCE_ID;
use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, SchoolId,
    SourceIdentity, SourceRef, Sport,
};
use census_domain::UsJurisdiction;

#[derive(Debug, Clone, Copy)]
pub struct ProfileFacts<'a> {
    pub name: &'a str,
    pub key: &'a str,
    pub url: &'a str,
    pub observed_on: &'a str,
    pub state: UsJurisdiction,
}

#[derive(Debug, Clone)]
pub struct SchoolExtract {
    pub school: CanonicalSchool,
    pub school_id: SchoolId,
    pub coaches: Vec<CanonicalCoach>,
}

pub fn school_entities(
    facts: &ProfileFacts<'_>,
    coaches: &[CoachRow],
    sport: Sport,
    gender: Gender,
) -> SchoolExtract {
    let normalized = normalize_name(facts.name);
    let (mut school, school_id) = CanonicalSchool::new(facts.state, facts.name, &normalized, None);

    school.association = Some(SOURCE_ID.to_string());
    school.source_identities.push(
        SourceIdentity::new(super::school_namespace(), facts.key.to_string())
            .with_url(facts.url.to_string()),
    );
    school.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(facts.url.to_string())),
        facts.observed_on.to_string(),
    ));

    let mut coach_entities: Vec<CanonicalCoach> = Vec::new();

    for row in coaches {
        if let Some(role) = map_role(&row.role) {
            if let Some(coach) = sport_coach(&school_id, facts, row, sport, gender, role) {
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

fn map_role(text: &str) -> Option<CoachRole> {
    let lower = text.to_lowercase();
    if lower == "head coach" || lower == "co-head coach" {
        Some(CoachRole::HeadCoach)
    } else if lower == "assistant coach" || lower == "volunteer coach" {
        Some(CoachRole::AssistantCoach)
    } else {
        None
    }
}

fn sport_coach(
    school_id: &SchoolId,
    facts: &ProfileFacts<'_>,
    row: &CoachRow,
    sport: Sport,
    gender: Gender,
    role: CoachRole,
) -> Option<CanonicalCoach> {
    let mut coach = CanonicalCoach::new(school_id, row.name.clone(), Some(sport), gender, role);

    coach.source_identities.push(
        SourceIdentity::new(
            super::school_namespace(),
            format!(
                "{}:{}:{}:{}:{}",
                facts.key,
                row.name,
                row.role,
                sport.stable_key(),
                gender.stable_key()
            ),
        )
        .with_url(facts.url.to_string()),
    );

    coach.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(facts.url.to_string())),
        facts.observed_on.to_string(),
    ));

    Some(coach)
}
