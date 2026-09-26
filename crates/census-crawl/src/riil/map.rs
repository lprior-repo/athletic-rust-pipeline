//! Canonical mapping: parsed rows → canonical entities from `census_domain::model`.
//!
//! Nothing here reads the network, the store, or a file: it takes parsed data from
//! [`super::pages`] and returns the canonical types.
//!
//! # School identity
//! The RIIL page publishes no numeric id for schools, so the join key is the normalized
//! school name alone (prefixed with the state code for the `Id` minting). Two different
//! spellings of the same school on the page will collide only if [`normalize_name`] maps them
//! to the same string — that is the same collision behaviour every other association adapter
//! relies on.

use census_domain::model::{
    CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, SchoolId, SourceIdentity,
    SourceNamespace, SourceRef,
};

use census_domain::model::normalize_name;

use super::ASSOCIATION;

/// One parsed row from a school's table.
#[derive(Debug, Clone)]
pub struct CoachRow {
    /// Raw sport label as rendered on the page (e.g. "Boys Cross Country").
    pub sport_label: String,
    /// Head coach name.
    pub coach_name: String,
    /// Published phone number, if present.
    pub phone: Option<String>,
    /// Canonical sport this label maps to.
    pub sport: census_domain::model::Sport,
}

/// One school extracted from the directory page.
#[derive(Debug, Clone)]
pub struct SchoolTable {
    /// School display name.
    pub name: String,
    /// All coach rows parsed from this school's table.
    pub coach_rows: Vec<CoachRow>,
}

/// Canonical entities for one school: one school row plus all its head coaches.
#[derive(Debug, Clone)]
pub struct SchoolExtract {
    pub school: CanonicalSchool,
    pub coaches: Vec<CanonicalCoach>,
}

/// Build canonical school and coach entities from a parsed school table.
pub fn school_entities(table: &SchoolTable, observed_on: &str) -> SchoolExtract {
    let (school, school_id) = school_from_table(table, observed_on);
    let mut coaches = Vec::new();

    for row in &table.coach_rows {
        let coach = coach_from_row(&school_id, row, observed_on);
        coaches.push(coach);
    }

    SchoolExtract { school, coaches }
}

/// Build the canonical school row from a parsed school table.
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

/// Build one canonical coach from a parsed coach row.
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

/// Derive gender from a sport label: "Boys" → Boys, "Girls" → Girls, "Coed" → Mixed.
fn gender_from_label(sport_label: &str) -> Gender {
    if sport_label.starts_with("Boys ") {
        Gender::Boys
    } else if sport_label.starts_with("Girls ") {
        Gender::Girls
    } else {
        Gender::Mixed
    }
}
