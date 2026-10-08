use crate::cli::export_data::csv::write_csv;
use census_domain::model::{CanonicalCoach, CanonicalSchool, CoachTenureEvidence};
use std::collections::{BTreeSet, HashMap};

fn build_coach_row(
    coach: &CanonicalCoach,
    schools: &HashMap<&str, &CanonicalSchool>,
) -> anyhow::Result<Vec<String>> {
    let mut row = identity(coach, schools.get(coach.school.as_str()).copied());
    let (source_url, observed_on) = census_report::export::coach_source(coach);
    row.extend([
        coach
            .professional_email
            .as_deref()
            .map_or("", |value| value)
            .to_owned(),
        coach
            .personal_email
            .as_deref()
            .map_or("", |value| value)
            .to_owned(),
        source_url.map_or("", |value| value).to_owned(),
        evidence_sources(coach),
        observed_on.map_or("", |value| value).to_owned(),
    ]);
    row.push(captures(coach, coach.professional_email.as_deref())?);
    row.push(captures(coach, coach.personal_email.as_deref())?);
    row.push(captures(coach, None)?);
    Ok(row)
}

fn identity(coach: &CanonicalCoach, school: Option<&CanonicalSchool>) -> Vec<String> {
    vec![
        coach.id.to_string(),
        coach.name.clone(),
        coach.role.stable_key().to_owned(),
        coach
            .sport
            .map_or("", |sport| sport.stable_key())
            .to_owned(),
        coach.gender.stable_key().to_owned(),
        coach.school.to_string(),
        school.map_or("", |school| school.name.as_str()).to_owned(),
        school
            .and_then(|school| school.state)
            .map_or("", |state| state.code())
            .to_owned(),
    ]
}

fn evidence_sources(coach: &CanonicalCoach) -> String {
    coach
        .evidence
        .iter()
        .map(|fact| fact.source.id.as_str())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(";")
}

fn captures(coach: &CanonicalCoach, mailbox: Option<&str>) -> anyhow::Result<String> {
    let facts: Vec<&CoachTenureEvidence> = coach
        .tenure_evidence
        .iter()
        .filter(|fact| {
            fact.claim.as_ref().is_some_and(|claim| {
                claim.coach == coach.id
                    && claim.school == coach.school
                    && claim.role == coach.role
                    && match mailbox {
                        Some(mailbox) => claim.mailbox.as_deref().is_some_and(|claimed| {
                            claimed.trim().eq_ignore_ascii_case(mailbox.trim())
                        }),
                        None => true,
                    }
            })
        })
        .collect();
    Ok(serde_json::to_string(&facts)?)
}

pub fn write_canonical_coaches(
    coaches: &[CanonicalCoach],
    schools: &[CanonicalSchool],
    data: &std::path::Path,
) -> anyhow::Result<()> {
    let schools: HashMap<_, _> = schools
        .iter()
        .map(|school| (school.id.as_str(), school))
        .collect();
    let rows = coaches
        .iter()
        .map(|coach| build_coach_row(coach, &schools))
        .collect::<anyhow::Result<Vec<_>>>()?;
    write_csv(
        &data.join("canonical-coaches.csv"),
        [
            "coach_id",
            "name",
            "role",
            "sport",
            "gender",
            "school_id",
            "school_name",
            "school_state",
            "professional_email",
            "personal_email",
            "archival_source_url",
            "evidence_sources",
            "archival_observed_on",
            "archival_professional_capture_claims",
            "archival_personal_capture_claims",
            "archival_name_capture_claims",
        ],
        &rows,
    )
}
