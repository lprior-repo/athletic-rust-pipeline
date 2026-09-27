use census_domain::model::{
    CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, SchoolId, SourceIdentity,
    SourceNamespace, SourceRef,
};

use census_domain::model::normalize_name;

use super::ASSOCIATION;

#[derive(Debug, Clone)]
pub struct CoachRow {
    pub sport_label: String,
    pub coach_name: String,
    pub phone: Option<String>,
    pub sport: census_domain::model::Sport,
}

#[derive(Debug, Clone)]
pub struct SchoolTable {
    pub name: String,
    pub coach_rows: Vec<CoachRow>,
}

#[derive(Debug, Clone)]
pub struct SchoolExtract {
    pub school: CanonicalSchool,
    pub coaches: Vec<CanonicalCoach>,
}

pub fn school_entities(table: &SchoolTable, observed_on: &str) -> SchoolExtract {
    let (school, school_id) = school_from_table(table, observed_on);
    let mut coaches = Vec::new();

    for row in &table.coach_rows {
        let coach = coach_from_row(&school_id, row, observed_on);
        coaches.push(coach);
    }

    SchoolExtract { school, coaches }
}

fn school_from_table(table: &SchoolTable, observed_on: &str) -> (CanonicalSchool, SchoolId) {
    let name = table.name.clone();
    let normalized = normalize_name(&name);
    let (school, id) = CanonicalSchool::new(super::STATE, name, normalized);
    let url = format!("{}/Directory.aspx", super::HOST);
    let evidence = Evidence::parsed(
        SourceRef::new(super::SOURCE_ID, Some(url.clone())),
        observed_on.to_string(),
    );
    let identity = SourceIdentity::new(
        SourceNamespace::association_school(ASSOCIATION),
        format!("school:{}", id),
    )
    .with_url(url);

    let mut school = school;
    school.evidence.push(evidence);
    school.source_identities.push(identity);

    (school, id)
}

fn coach_from_row(school_id: &SchoolId, row: &CoachRow, observed_on: &str) -> CanonicalCoach {
    let gender = gender_from_label(&row.sport_label);
    let mut coach = CanonicalCoach::new(
        school_id,
        &row.coach_name,
        Some(row.sport),
        gender,
        CoachRole::HeadCoach,
    );

    if let Some(phone) = &row.phone {
        coach.phone = Some(phone.clone());
    }

    let url = format!("{}/Directory.aspx", super::HOST);
    coach.source_identities.push(
        SourceIdentity::new(
            SourceNamespace::AssociationSchool {
                association: ASSOCIATION.to_string(),
            },
            format!(
                "coach:{}:{}:{}:{}",
                school_id,
                row.sport.stable_key(),
                gender.stable_key(),
                "HeadCoach"
            ),
        )
        .with_url(url),
    );
    coach.evidence.push(Evidence::parsed(
        SourceRef::new(
            super::SOURCE_ID,
            Some(format!("{}/Directory.aspx", super::HOST)),
        ),
        observed_on.to_string(),
    ));

    coach
}

fn gender_from_label(sport_label: &str) -> Gender {
    if sport_label.starts_with("Boys ") {
        Gender::Boys
    } else if sport_label.starts_with("Girls ") {
        Gender::Girls
    } else {
        Gender::Mixed
    }
}
