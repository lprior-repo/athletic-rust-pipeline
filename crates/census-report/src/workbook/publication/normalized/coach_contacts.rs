use super::{encode, provenance, text, write_csv};
use crate::report::{Derivation, ReportResult};
use crate::workbook::recruiting::{bounded, coach_projection};
use census_domain::model::{CanonicalCoach, CoachRole, SchoolYear};
use std::path::Path;

const HEADER: [&str; 22] = [
    "contact_role_id",
    "school_id",
    "published_name",
    "role",
    "sports",
    "divisions",
    "work_email",
    "work_phone",
    "contact_url",
    "status",
    "tenure_status",
    "effective_from",
    "effective_until",
    "last_verification",
    "source_url",
    "fallback_type",
    "evidence_refs",
    "capture_content_sha256",
    "retrieved_at",
    "retained_tenure_claims",
    "input_generation",
    "unavailable_fields_status",
];

pub(super) fn claims(derivation: &Derivation<'_>) -> ReportResult<Vec<CanonicalCoach>> {
    bounded::bounded_claims(
        derivation.coach_observations(),
        bounded::WORKBOOK_GROUP_BUDGET,
    )
}

pub(super) fn write(
    directory: &Path,
    derivation: &Derivation<'_>,
    year: SchoolYear,
) -> ReportResult<()> {
    let coaches = claims(derivation)?;
    let contacts = bounded::bounded_contacts(&coaches, year, bounded::WORKBOOK_GROUP_BUDGET)?;
    write_csv(&directory.join("coach_contacts.csv"), &HEADER, |writer| {
        for coach in &coaches {
            let projected =
                coach_projection::admitted(coach, year, contacts.get(coach.school.as_str()));
            writer.push(row(
                coach,
                projected,
                year,
                &derivation.dataset().lineage.input_generation,
            )?)?;
        }
        Ok(())
    })
}

fn row(
    coach: &CanonicalCoach,
    projected: coach_projection::Projection<'_>,
    year: SchoolYear,
    generation: &str,
) -> ReportResult<[String; 22]> {
    let fact = projected
        .professional
        .map(|selected| selected.tenure())
        .or(projected.name);
    let [url, sha, retrieved] = coach_projection::capture(fact);
    let source_url = if url.is_empty() {
        crate::export::coach_source(coach).0
    } else {
        Some(url)
    };
    Ok([
        coach.id.to_string(),
        coach.school.to_string(),
        text(Some(&coach.name)),
        coach.role.stable_key().into(),
        coach
            .sport
            .map_or_else(|| "unknown".into(), |sport| sport.stable_key().into()),
        coach.gender.stable_key().into(),
        channel(
            projected.professional.map(|selected| selected.mailbox()),
            coach.professional_email.as_deref(),
        ),
        channel(None, coach.phone.as_deref()),
        "unavailable".into(),
        projected.state.into(),
        coach_projection::tenure_label(coach, year).into(),
        "unavailable".into(),
        "unavailable".into(),
        "unavailable".into(),
        text(source_url),
        if coach.role == CoachRole::AthleticDirector {
            "athletic_director"
        } else {
            "not_applicable"
        }
        .into(),
        provenance::references("coach", coach.id.as_str(), &coach.evidence)?,
        text(Some(sha)),
        text(Some(retrieved)),
        encode(&coach.tenure_evidence)?,
        generation.into(),
        "phone_contact_url_effective_dates_verification_not_retained_as_admitted_claims".into(),
    ])
}

fn channel(admitted: Option<&str>, retained: Option<&str>) -> String {
    match admitted {
        Some(value) => text(Some(value)),
        None if retained.is_some_and(|value| !value.trim().is_empty()) => {
            "withheld_not_admitted".into()
        }
        None => "unavailable".into(),
    }
}
