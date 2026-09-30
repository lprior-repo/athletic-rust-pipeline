use crate::report::ReportResult;
use census_domain::model::CanonicalCoach;

use super::super::cells::{row, Cell};
use super::dataset::Dataset;

pub(super) const TITLE: &str = "Coaches";

pub(super) const HEADERS: [&str; 16] = [
    "School ID",
    "School",
    "School City",
    "State",
    "Sport",
    "Coach",
    "Role",
    "Professional Email",
    "Personal Email",
    "Phone",
    "Athletic Director",
    "AD Professional Email",
    "Official Source URL",
    "Observed Date",
    "Declared Tenure",
    "Assessment School Year",
];

pub(super) const WIDTHS: [u16; 16] = [
    20, 30, 20, 8, 14, 26, 16, 32, 32, 18, 26, 32, 40, 14, 24, 22,
];

pub(super) fn sheet(dataset: &Dataset) -> ReportResult<Vec<Vec<Cell>>> {
    let mut ordered: Vec<(&CanonicalCoach, SortKey)> = dataset
        .coaches
        .iter()
        .map(|coach| (coach, sort_key(dataset, coach)))
        .collect();
    ordered.sort_by(|left, right| left.1.cmp(&right.1));
    let mut rows = vec![HEADERS.iter().map(|header| Cell::text(*header)).collect()];
    for (coach, _) in ordered {
        rows.push(row_for(dataset, coach));
    }
    Ok(rows)
}

type SortKey = (String, String, String, String, String);

fn sort_key(dataset: &Dataset, coach: &CanonicalCoach) -> SortKey {
    (
        dataset.school_state(coach.school.as_str()).to_string(),
        dataset.school_name(coach.school.as_str()).to_string(),
        sport_label(coach),
        role_label(coach),
        coach.name.clone(),
    )
}

fn row_for(dataset: &Dataset, coach: &CanonicalCoach) -> Vec<Cell> {
    let director = dataset
        .contacts
        .get(coach.school.as_str())
        .and_then(|contacts| contacts.director());
    row!(
        Cell::text(coach.school.as_str()),
        Cell::text(dataset.school_name(coach.school.as_str())),
        Cell::text(dataset.school_city(coach.school.as_str())),
        Cell::text(dataset.school_state(coach.school.as_str())),
        Cell::text(sport_label(coach)),
        Cell::text(coach.name.clone()),
        Cell::text(role_label(coach)),
        Cell::text(coach.professional_email.clone().unwrap_or_default()),
        Cell::text(coach.personal_email.clone().unwrap_or_default()),
        Cell::text(coach.phone.clone().unwrap_or_default()),
        Cell::text(
            director
                .map(|director| director.name.clone())
                .unwrap_or_default(),
        ),
        Cell::text(
            director
                .and_then(|director| director.email.clone())
                .unwrap_or_default(),
        ),
        Cell::text(official_url(coach)),
        Cell::text(observed_date(coach)),
        Cell::text(tenure_label(coach, dataset.school_year)),
        Cell::text(dataset.school_year.short()),
    )
}

fn tenure_label(
    coach: &CanonicalCoach,
    school_year: census_domain::model::SchoolYear,
) -> &'static str {
    use census_domain::model::{CoachTenure, TenureAssessmentError};
    match coach.tenure_state(school_year) {
        Ok(CoachTenure::Current { .. }) => "current_declared",
        Ok(CoachTenure::Former { .. }) => "former_declared",
        Ok(CoachTenure::Unknown) => "unknown",
        Err(TenureAssessmentError::Conflict) => "tenure_conflict",
        Err(TenureAssessmentError::InvalidEvidence { .. }) => "invalid_tenure_evidence",
    }
}

fn sport_label(coach: &CanonicalCoach) -> String {
    match (coach.sport, coach.role) {
        (Some(sport), _) => sport.stable_key().to_owned(),
        (None, census_domain::model::CoachRole::AthleticDirector) => "school_wide".to_owned(),
        (None, _) => "unknown".to_owned(),
    }
}

fn role_label(coach: &CanonicalCoach) -> String {
    coach.role.stable_key().to_string()
}

fn official_url(coach: &CanonicalCoach) -> String {
    coach
        .evidence
        .iter()
        .find_map(|evidence| evidence.source.url.clone())
        .or_else(|| {
            coach
                .source_identities
                .iter()
                .find_map(|identity| identity.url.clone())
        })
        .unwrap_or_default()
}

fn observed_date(coach: &CanonicalCoach) -> String {
    coach
        .evidence
        .iter()
        .map(|evidence| evidence.observed_on.as_str())
        .max()
        .unwrap_or_default()
        .to_string()
}
