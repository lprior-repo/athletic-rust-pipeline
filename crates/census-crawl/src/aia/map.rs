use super::parse::{parse_gender, parse_sport_label, SchoolProfile};
use super::{ASSOCIATION, HOST, SOURCE_ID, STATE};
use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, SchoolId, SourceIdentity,
    SourceNamespace, SourceRef,
};

#[derive(Debug, Clone)]
pub struct SchoolExtract {
    pub school: CanonicalSchool,
    pub school_id: SchoolId,
    pub coaches: Vec<CanonicalCoach>,
}

pub fn school_entities(
    school_name: &str,
    aia_id: &str,
    city: Option<&str>,
    profile: &SchoolProfile,
    observed_on: &str,
) -> SchoolExtract {
    let normalized = normalize_name(school_name);
    let (mut school, school_id) = CanonicalSchool::new(
        STATE,
        school_name,
        &normalized,
        city.map(str::trim).filter(|value| !value.is_empty()),
    );
    school.association = Some(ASSOCIATION.to_string());

    let school_url = format!("{HOST}/schools/{aia_id}");
    school.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(school_url.clone())),
        observed_on.to_string(),
    ));

    let mut coaches = Vec::new();
    for row in &profile.coaches {
        if let Some(sport) = parse_sport_label(&row.sport_label) {
            let gender = parse_gender(&row.sport_label);
            if let Some(coach) = sport_coach(
                &school_id,
                &row.sport_label,
                &row.name,
                sport,
                gender,
                &school_url,
                observed_on,
            ) {
                coaches.push(coach);
            }
        }
    }

    SchoolExtract {
        school,
        school_id,
        coaches,
    }
}

fn sport_coach(
    school_id: &SchoolId,
    sport_label: &str,
    coach_name: &str,
    sport: census_domain::model::Sport,
    gender: census_domain::model::Gender,
    url: &str,
    observed_on: &str,
) -> Option<CanonicalCoach> {
    if coach_name.is_empty() {
        return None;
    }

    let mut coach = CanonicalCoach::new(
        school_id,
        coach_name.to_string(),
        Some(sport),
        gender,
        CoachRole::HeadCoach,
    );

    coach.source_identities.push(
        SourceIdentity::new(
            SourceNamespace::AssociationSchool {
                association: ASSOCIATION.to_string(),
            },
            format!("coach:{}:{}", sport_label, coach_name),
        )
        .with_url(url.to_string()),
    );

    coach.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(url.to_string())),
        observed_on.to_string(),
    ));

    Some(coach)
}
