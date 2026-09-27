use super::pages::{parse_gender, parse_sport_label};
use super::{ASSOCIATION, HOST, SOURCE_ID, STATE};
use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, SchoolId,
    SourceIdentity, SourceNamespace, SourceRef, Sport,
};

#[derive(Debug, Clone, Default)]
pub struct SchoolTable {
    pub rows: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct SchoolExtract {
    pub school: CanonicalSchool,
    pub school_id: SchoolId,
    pub coaches: Vec<CanonicalCoach>,
}

pub fn school_entities(school_name: &str, table: &SchoolTable, observed_on: &str) -> SchoolExtract {
    let normalized = normalize_name(school_name);
    let (mut school, school_id) = CanonicalSchool::new(STATE, school_name, &normalized);
    school.association = Some(ASSOCIATION.to_string());
    school.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(format!("{HOST}/Directory.aspx"))),
        observed_on.to_string(),
    ));

    let mut coaches: Vec<CanonicalCoach> = Vec::new();

    for (sport_label, coach_name) in &table.rows {
        if let Some(sport) = parse_sport_label(sport_label) {
            let gender = parse_gender(sport_label);
            if let Some(row) = sport_coach(
                &school_id,
                sport_label,
                coach_name,
                sport,
                gender,
                observed_on,
            ) {
                coaches.push(row);
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
    sport: Sport,
    gender: Gender,
    observed_on: &str,
) -> Option<CanonicalCoach> {
    if super::pages::is_placeholder_name(coach_name) {
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
            format!("coach:{sport_label}"),
        )
        .with_url(format!("{HOST}/Directory.aspx")),
    );
    coach.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(format!("{HOST}/Directory.aspx"))),
        observed_on.to_string(),
    ));
    Some(coach)
}
