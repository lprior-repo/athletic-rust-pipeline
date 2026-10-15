use super::super::input::Inputs;
use super::{
    facts,
    rows::{self, Row},
    schema,
    schools::Schools,
};
use crate::report::ReportResult;
use crate::workbook::recruiting::{bounded, coach_projection};
use census_domain::model::{CanonicalCoach, CoachRole};
use std::collections::BTreeMap;
use std::path::Path;

pub(super) fn verify(
    directory: &Path,
    inputs: &Inputs<'_>,
    coaches: &[CanonicalCoach],
    schools: &Schools<'_>,
) -> ReportResult<u64> {
    let roles: BTreeMap<_, _> = coaches
        .iter()
        .map(|coach| (coach.id.as_str(), coach))
        .collect();
    let contacts =
        bounded::bounded_contacts(coaches, inputs.school_year, bounded::WORKBOOK_GROUP_BUDGET)?;
    rows::table(
        directory,
        ("coach_contacts.csv", schema::CONTACTS),
        roles.len(),
        &["contact_role_id"],
        |row| {
            let id = row.get("contact_role_id")?;
            let coach = roles
                .get(id)
                .copied()
                .ok_or_else(|| rows::missing("contact role", id))?;
            if !matches!(
                schools.get(coach.school.as_str()),
                Some(super::schools::Reference::Resolved(_))
            ) {
                return Err(rows::missing("contact school", coach.school.as_str()));
            }
            row.text("school_id", coach.school.as_str())?;
            row.text("published_name", rows::available(Some(&coach.name)))?;
            row.text("role", coach.role.stable_key())?;
            row.text(
                "sports",
                coach.sport.map_or("unknown", |sport| sport.stable_key()),
            )?;
            row.text("divisions", coach.gender.stable_key())?;
            row.text(
                "fallback_type",
                if coach.role == CoachRole::AthleticDirector {
                    "athletic_director"
                } else {
                    "not_applicable"
                },
            )?;
            let selected = coach_projection::admitted(
                coach,
                inputs.school_year,
                contacts.get(coach.school.as_str()),
            );
            channels(row, coach, selected)?;
            row.text(
                "tenure_status",
                coach_projection::tenure_label(coach, inputs.school_year),
            )?;
            row.text("input_generation", &inputs.dataset.lineage.input_generation)
        },
    )
}

fn channels(
    row: &Row<'_>,
    coach: &CanonicalCoach,
    selected: coach_projection::Projection<'_>,
) -> ReportResult<()> {
    row.text("status", selected.state)?;
    row.text(
        "work_email",
        channel(
            selected.professional.map(|contact| contact.mailbox()),
            coach.professional_email.as_deref(),
        ),
    )?;
    row.text("work_phone", channel(None, coach.phone.as_deref()))?;
    let fact = selected
        .professional
        .map(|contact| contact.tenure())
        .or(selected.name);
    let [url, sha, retrieved] = coach_projection::capture(fact);
    let url = if url.is_empty() {
        crate::export::coach_source(coach).0
    } else {
        Some(url)
    };
    row.text("source_url", rows::available(url))?;
    row.text("capture_content_sha256", rows::available(Some(sha)))?;
    row.text("retrieved_at", rows::available(Some(retrieved)))?;
    row.json("retained_tenure_claims", &coach.tenure_evidence)?;
    row.json(
        "evidence_refs",
        &facts::references("coach", coach.id.as_str(), &coach.evidence)?,
    )?;
    row.literals(&[
        ("contact_url", "unavailable"),
        ("effective_from", "unavailable"),
        ("effective_until", "unavailable"),
        ("last_verification", "unavailable"),
        (
            "unavailable_fields_status",
            "phone_contact_url_effective_dates_verification_not_retained_as_admitted_claims",
        ),
    ])
}

fn channel<'a>(admitted: Option<&'a str>, retained: Option<&str>) -> &'a str {
    match admitted {
        Some(value) => rows::available(Some(value)),
        None if retained.is_some_and(|value| !value.trim().is_empty()) => "withheld_not_admitted",
        None => "unavailable",
    }
}
