use super::cen15::{captured_coach, ACQUIRED_A, CAPTURE_A, MAILBOX_A};
use super::*;
use calamine::{open_workbook, Reader, Xlsx};
use census_domain::model::{CanonicalSchool, Evidence, PublishedGraduation};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn seed_population(store: &Store) -> TestResult<SchoolId> {
    let (mut school, owner) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "CEN15 High School",
        "cen15 high school",
        Some("Census City"),
    );
    let source = SourceRef::new(
        "wiaa_results",
        Some("https://example.invalid/roster".into()),
    );
    school
        .evidence
        .push(Evidence::parsed(source.clone(), "2026-08-01"));
    store.append(Table::Schools, &school)?;
    let mut athlete = CanonicalAthlete::new(
        &owner,
        "CEN15 Runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::Other("fixture".into()), "cen15-runner"),
    );
    athlete.sports = vec![Sport::OutdoorTrack];
    athlete
        .evidence
        .push(Evidence::parsed(source.clone(), "2026-08-01"));
    athlete.published_graduations.push(PublishedGraduation {
        grad_year: GradYear::CO2027,
        source,
    });
    store.append(Table::Athletes, &athlete)?;
    Ok(owner)
}

fn publish(store: &Store) -> TestResult<PathBuf> {
    Ok(crate::workbook::build(
        store,
        &crate::workbook::Options {
            grad_year: Some(2027),
            out: None,
            limit: None,
            scope: crate::report::Scope::AllSources,
            school_year: year()?,
        },
    )?)
}

fn workbook_row(
    path: &Path,
    sheet: &str,
    field: &str,
    value: &str,
) -> TestResult<BTreeMap<String, String>> {
    let mut workbook: Xlsx<_> = open_workbook(path)?;
    let range = workbook.worksheet_range(sheet)?;
    let mut rows = range.rows();
    let headers = rows.next().ok_or("missing workbook headers")?;
    let column = headers
        .iter()
        .position(|header| header.to_string() == field)
        .ok_or("missing identity header")?;
    let row = rows
        .find(|row| {
            row.get(column)
                .is_some_and(|cell| cell.to_string() == value)
        })
        .ok_or("missing published athlete")?;
    Ok(headers
        .iter()
        .zip(row)
        .map(|(header, cell)| (header.to_string(), cell.to_string()))
        .collect())
}

fn csv_row(path: &Path) -> TestResult<BTreeMap<String, String>> {
    let mut csv = csv::Reader::from_path(
        path.parent()
            .ok_or("generation parent")?
            .join("recruiting.csv"),
    )?;
    let headers = csv.headers()?.clone();
    let mut rows = csv.records();
    let row = rows.next().ok_or("missing recruiting row")??;
    check!(eq; rows.next().transpose()?, None);
    Ok(headers
        .iter()
        .zip(row.iter())
        .map(|(header, value)| (header.to_owned(), value.to_owned()))
        .collect())
}

fn field<'a>(row: &'a BTreeMap<String, String>, key: &str) -> TestResult<&'a str> {
    row.get(key)
        .map(String::as_str)
        .ok_or_else(|| format!("missing published field {key}").into())
}

fn assert_capture_json(row: &BTreeMap<String, String>, coach: &CanonicalCoach) -> TestResult {
    let json: serde_json::Value = serde_json::from_str(field(row, "contact_provenance")?)?;
    let entries = json.as_array().ok_or("contact provenance array")?;
    check!(eq; entries.len(), 1);
    let capture = entries.first().ok_or("mailbox capture")?;
    check!(eq; capture.get("coach_id"), Some(&serde_json::json!(coach.id.as_str())));
    check!(eq; capture.get("mailbox"), Some(&serde_json::json!(MAILBOX_A)));
    check!(eq; capture.get("source_url"), Some(&serde_json::json!(CAPTURE_A)));
    check!(eq; capture.get("source_sha256"), Some(&serde_json::json!("a".repeat(64))));
    check!(eq; capture.get("acquired_at"), Some(&serde_json::json!(ACQUIRED_A)));
    Ok(())
}

#[test]
fn cen15_actual_publication_uses_mailbox_capture_not_newer_generic_evidence() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    let owner = seed_population(&store)?;
    let coach = captured_coach(&owner)?;
    store.append(Table::Coaches, &coach)?;
    let path = publish(&store)?;
    let row = workbook_row(&path, "Athletes", "Name", "CEN15 Runner")?;
    check!(eq; field(&row, "Preferred Contact Email")?, MAILBOX_A);
    check!(eq; field(&row, "Preferred Contact Coach ID")?, coach.id.as_str());
    check!(eq; field(&row, "Preferred Contact Source URL")?, CAPTURE_A);
    check!(eq; field(&row, "Preferred Contact Capture SHA256")?, "a".repeat(64));
    check!(eq; field(&row, "Preferred Contact Acquired At")?, ACQUIRED_A);
    let row = csv_row(&path)?;
    check!(eq; field(&row, "head_track_coach_email")?, MAILBOX_A);
    check!(eq; field(&row, "coach_id")?, coach.id.as_str());
    check!(eq; field(&row, "coach_source_url")?, CAPTURE_A);
    check!(eq; field(&row, "coach_capture_sha256")?, "a".repeat(64));
    check!(eq; field(&row, "coach_acquired_at")?, ACQUIRED_A);
    assert_capture_json(&row, &coach)?;
    crate::workbook::publication::verify_published(&path)?;
    Ok(())
}

#[test]
fn cen15_actual_publication_withholds_conflicting_mailboxes_and_their_provenance() -> TestResult {
    for reverse in [false, true] {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path().join("store"))?;
        let owner = seed_population(&store)?;
        let first = captured_coach(&owner)?;
        let mut second = first.clone();
        second.professional_email = Some("other@example.invalid".into());
        second.tenure_evidence.retain(|fact| {
            fact.claim
                .as_ref()
                .is_some_and(|claim| claim.mailbox.is_some())
        });
        second
            .tenure_evidence
            .first_mut()
            .ok_or("conflicting fact")?
            .claim
            .as_mut()
            .ok_or("conflicting claim")?
            .mailbox = second.professional_email.clone();
        let rows = if reverse {
            [second, first]
        } else {
            [first, second]
        };
        for coach in rows {
            store.append(Table::Coaches, &coach)?;
        }
        let path = publish(&store)?;
        let row = workbook_row(&path, "Athletes", "Name", "CEN15 Runner")?;
        check!(eq; field(&row, "Contact Coverage State")?, "contact_conflict");
        for header in [
            "Preferred Contact Email",
            "Preferred Contact Coach ID",
            "Preferred Contact Source URL",
            "Preferred Contact Capture SHA256",
            "Preferred Contact Acquired At",
            "Head TF Coach Email",
            "All Emails (school)",
        ] {
            check!(eq; field(&row, header)?, "");
        }
        let row = csv_row(&path)?;
        for header in [
            "head_track_coach_email",
            "coach_id",
            "coach_source_url",
            "coach_capture_sha256",
            "coach_acquired_at",
        ] {
            check!(eq; field(&row, header)?, "");
        }
        let json: serde_json::Value = serde_json::from_str(field(&row, "contact_provenance")?)?;
        check!(eq; json, serde_json::json!([]));
        crate::workbook::publication::verify_published(&path)?;
    }
    Ok(())
}
