use crate::net::FetchOutcome;
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

pub fn school_entities(table: &SchoolTable, capture: &FetchOutcome) -> SchoolExtract {
    let (school, school_id) = school_from_table(table, capture);
    let coaches = table
        .coach_rows
        .iter()
        .map(|row| coach_from_row(&school_id, row, capture))
        .collect();
    SchoolExtract { school, coaches }
}

fn school_from_table(table: &SchoolTable, capture: &FetchOutcome) -> (CanonicalSchool, SchoolId) {
    let name = table.name.clone();
    let normalized = normalize_name(&name);
    let (school, id) = CanonicalSchool::new(super::STATE, name, normalized);
    let evidence = capture_evidence(capture);
    let identity = SourceIdentity::new(
        SourceNamespace::association_school(ASSOCIATION),
        format!("school:{}", id),
    )
    .with_url(capture.url.clone());

    let mut school = school;
    school.evidence.push(evidence);
    school.source_identities.push(identity);

    (school, id)
}

fn coach_from_row(school_id: &SchoolId, row: &CoachRow, capture: &FetchOutcome) -> CanonicalCoach {
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

    coach.evidence.push(capture_evidence(capture));

    coach
}

pub(super) fn capture_note(capture: &FetchOutcome) -> serde_json::Value {
    serde_json::json!({
        "capture_url": capture.url,
        "sha256": capture.content_digest,
        "acquired_at": capture.fetched_at,
    })
}

fn capture_evidence(capture: &FetchOutcome) -> Evidence {
    let mut evidence = Evidence::parsed(
        SourceRef::new(super::SOURCE_ID, Some(capture.url.clone())),
        &capture.fetched_at,
    );
    evidence.note = Some(capture_note(capture).to_string());
    evidence
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
