use super::super::{files, read, TestResult};
use super::fixture::{Positive, ADDRESS_URL, DAY, RETRIEVED, STAFF_URL};
use census_domain::model::CoachRole;
use std::path::Path;

pub(super) const CONTACTS: [&str; 22] = [
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

pub(super) const SCHOOLS: [&str; 26] = [
    "school_id",
    "canonical_name",
    "aliases",
    "school_type",
    "school_status",
    "address_status",
    "address_verification_status",
    "evidence_refs",
    "input_generation",
    "record_revision_status",
    "postal_school_id",
    "postal_street",
    "postal_second_line",
    "postal_city",
    "postal_state",
    "postal_zip",
    "postal_owner_namespace",
    "postal_owner_id",
    "postal_source",
    "postal_source_url",
    "postal_observed_date",
    "postal_capture_sha256",
    "postal_address_kind",
    "address_verification_date",
    "address_claim_ids",
    "address_evidence_refs",
];

pub(super) const ALIASES: [&str; 12] = [
    "original_id",
    "canonical_id",
    "entity_type",
    "resolution_state",
    "successor_ids",
    "effective_revision",
    "resolution_reference",
    "input_generation",
    "publication_membership",
    "historical_state_status",
    "resolved_id",
    "resolution_reference_status",
];

pub(super) fn contacts(given: &Positive) -> TestResult<Vec<Vec<String>>> {
    given
        .population
        .coaches
        .iter()
        .map(|coach| {
            let role = match coach.role {
                CoachRole::HeadCoach => "HeadCoach",
                CoachRole::AssistantCoach => "AssistantCoach",
                other => return Err(format!("unexpected positive role {other:?}").into()),
            };
            let fact = coach.evidence.first().ok_or("missing coach evidence")?;
            Ok(strings(&[
                coach.id.as_str(),
                given.population.school.id.as_str(),
                role,
                role,
                "OutdoorTrack",
                "Girls",
                "unadmitted@example.test",
                "withheld_not_admitted",
                "unavailable",
                "current_claim",
                "current_declared",
                "unavailable",
                "unavailable",
                "unavailable",
                STAFF_URL,
                "not_applicable",
                &json(&[digest(&("coach", coach.id.as_str(), fact))?])?,
                &"c".repeat(64),
                RETRIEVED,
                &json(&coach.tenure_evidence)?,
                &given.population.dataset.lineage.input_generation,
                "phone_contact_url_effective_dates_verification_not_retained_as_admitted_claims",
            ]))
        })
        .collect()
}

pub(super) fn schools(given: &Positive) -> TestResult<Vec<Vec<String>>> {
    let claim = digest(&given.address)?;
    let evidence = digest(&("school_address", claim.as_str(), given.address.evidence()))?;
    Ok(vec![strings(&[
        given.population.school.id.as_str(),
        "Readback High",
        "[]",
        "institution_type_unavailable",
        "currentness_unavailable",
        "retained_owner_bound_claims",
        "observed_date_is_not_verification_date",
        "[]",
        &given.population.dataset.lineage.input_generation,
        "record_revision_not_retained",
        given.population.school.id.as_str(),
        "123 Runner Way",
        "Athletics Office",
        "Lansing",
        "MI",
        "48901",
        "association_school:mhsaa",
        "readback-high",
        "athletic-association:MI",
        ADDRESS_URL,
        DAY,
        &"b".repeat(64),
        "physical",
        "unavailable",
        &json(&[claim])?,
        &json(&[evidence])?,
    ])])
}

pub(super) fn aliases(given: &Positive) -> TestResult<Vec<Vec<String>>> {
    let population = &given.population;
    let mut records = Vec::new();
    for (id, included, merged) in [
        (&population.athletes[0].id, true, false),
        (&population.athletes[1].id, true, false),
        (&population.outside, false, false),
        (&given.members[0].id, true, true),
        (&given.members[1].id, true, true),
    ] {
        let resolved = if merged { given.canonical() } else { id };
        let alias = merged && id == given.alias();
        records.push(strings(&[
            id.as_str(),
            if included {
                resolved.as_str()
            } else {
                "unavailable"
            },
            "athlete",
            if merged { "verified" } else { "unverified" },
            "unavailable",
            "unavailable",
            &if alias {
                json(&[&given.decision])?
            } else {
                "[]".into()
            },
            &population.dataset.lineage.input_generation,
            if included {
                "canonical_in_publication"
            } else {
                "canonical_outside_publication"
            },
            "effective_revision_split_successors_supersession_not_retained",
            resolved.as_str(),
            if alias {
                "retained_projection_transitions"
            } else {
                "no_retained_alias_transition"
            },
        ]));
    }
    Ok(records)
}

pub(super) fn compare_and_replace(
    directory: &Path,
    family: &str,
    header: &[&str],
    expected: &[Vec<String>],
) -> TestResult {
    let (actual_header, mut actual) = read(directory, family)?;
    check!(eq; actual_header, strings(header));
    let mut wanted = expected.to_vec();
    actual.sort();
    wanted.sort();
    check!(eq; actual, wanted, "{family}");
    files::put(directory, family, header, expected)
}

pub(super) fn json(value: &impl serde::Serialize) -> TestResult<String> {
    Ok(serde_json::to_string(value)?)
}

pub(super) fn digest(value: &impl serde::Serialize) -> TestResult<String> {
    Ok(census_domain::model::serialized_digest(value)?)
}

pub(super) fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).into()).collect()
}
