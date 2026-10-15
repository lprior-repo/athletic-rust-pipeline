use super::super::{files, read, rejected, TestResult};
use super::{column, fixture::Positive, prepare, rows};
use crate::report::ReportError;
use std::path::Path;

#[test]
fn admitted_contact_mutations_cannot_substitute_mailbox_tenure_or_capture() -> TestResult {
    let given = Positive::new()?;
    let id = given
        .population
        .coaches
        .first()
        .ok_or("missing coach")?
        .id
        .as_str();
    mutations(
        &given,
        "coach_contacts.csv",
        "contact_role_id",
        id,
        &[
            ("work_email", "other@example.test"),
            ("work_phone", "555-0100"),
            ("status", "no_current_claim"),
            ("tenure_status", "unknown"),
            ("source_url", "https://foreign.example.test/staff"),
            ("capture_content_sha256", "foreign-capture"),
            ("retrieved_at", "2026-09-02T12:00:00Z"),
            ("retained_tenure_claims", "[]"),
            ("evidence_refs", "[]"),
            ("last_verification", "2026-09-01"),
        ],
    )
}

#[test]
fn owner_bound_postal_mutations_cannot_change_address_capture_or_evidence_links() -> TestResult {
    let given = Positive::new()?;
    mutations(
        &given,
        "schools.csv",
        "school_id",
        given.population.school.id.as_str(),
        &[
            ("postal_street", "999 Other Road"),
            ("postal_city", "Detroit"),
            ("postal_state", "WI"),
            ("postal_zip", "48902"),
            ("postal_owner_namespace", "association_school:wiaa"),
            ("postal_owner_id", "other-high"),
            ("postal_source_url", "https://foreign.example.test/school"),
            ("postal_observed_date", "2026-09-02"),
            ("postal_capture_sha256", "foreign-capture"),
            ("address_claim_ids", "[]"),
            ("address_evidence_refs", "[]"),
            ("address_verification_date", "2026-09-01"),
        ],
    )
}

#[test]
fn merged_alias_mutations_cannot_remove_redirect_or_decision_provenance() -> TestResult {
    let given = Positive::new()?;
    mutations(
        &given,
        "identity_aliases.csv",
        "original_id",
        given.alias().as_str(),
        &[
            ("canonical_id", given.alias().as_str()),
            ("resolved_id", given.alias().as_str()),
            ("resolution_state", "unverified"),
            ("resolution_reference", "[]"),
            (
                "resolution_reference_status",
                "no_retained_alias_transition",
            ),
            ("publication_membership", "canonical_outside_publication"),
            ("effective_revision", "123"),
            ("successor_ids", "[]"),
        ],
    )
}

#[test]
fn postal_provenance_mutations_cannot_invent_capture_edges_or_replace_retained_fact() -> TestResult
{
    let given = Positive::new()?;
    let owner = rows::digest(&given.address)?;
    let evidence = rows::digest(&("school_address", owner.as_str(), given.address.evidence()))?;
    mutations(
        &given,
        "provenance.csv",
        "evidence_id",
        &evidence,
        &[
            ("source_url", "https://foreign.example.test/school"),
            ("capture_id", "invented-capture"),
            ("capture_time", "2026-09-01T12:00:00Z"),
            ("interpretation_version", "invented-parser"),
            ("observed_on", "2026-09-02"),
        ],
    )?;
    let directory = tempfile::tempdir()?;
    prepare(&given, directory.path())?;
    replace(
        directory.path(),
        "provenance.csv",
        "evidence_id",
        &evidence,
        "retained_fact",
        "{}",
    )?;
    match super::super::super::verify(directory.path(), &given.population.inputs()?, 6) {
        Err(ReportError::Invariant { detail }) => {
            check!(eq; detail, "normalized retained evidence fact differs from frozen input");
        }
        other => {
            return Err(format!("postal retained fact mutation not refused: {other:?}").into())
        }
    }
    Ok(())
}

fn mutations(
    given: &Positive,
    family: &str,
    key: &str,
    id: &str,
    changes: &[(&str, &str)],
) -> TestResult {
    let directory = tempfile::tempdir()?;
    for (field, replacement) in changes {
        prepare(given, directory.path())?;
        let position = column(&read(directory.path(), family)?.0, field)?;
        replace(directory.path(), family, key, id, field, replacement)?;
        rejected(
            super::super::super::verify(directory.path(), &given.population.inputs()?, 6),
            family,
            &format!("column {position}:"),
        )?;
    }
    Ok(())
}

fn replace(
    directory: &Path,
    family: &str,
    key: &str,
    id: &str,
    field: &str,
    replacement: &str,
) -> TestResult {
    let (header, mut records) = read(directory, family)?;
    let key = column(&header, key)?;
    let field = column(&header, field)?;
    let row = records
        .iter_mut()
        .find(|row| row.get(key).is_some_and(|value| value == id))
        .ok_or("missing positive mutation row")?;
    *row.get_mut(field).ok_or("missing positive mutation cell")? = replacement.into();
    files::put(
        directory,
        family,
        &header.iter().map(String::as_str).collect::<Vec<_>>(),
        &records,
    )
}
