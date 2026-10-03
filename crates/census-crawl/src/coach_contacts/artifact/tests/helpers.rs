use super::super::{stage_verified_contacts, Manifest, CONTACT_COLUMNS};
use census_domain::model::{ContactClaimEvidence, ContactProofField, RawContactRow};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub fn sample_row() -> RawContactRow {
    RawContactRow {
        school: "Test High".to_owned(),
        city: "Springfield".to_owned(),
        state: "IL".to_owned(),
        sport: "Boys Track".to_owned(),
        role: "Head Coach".to_owned(),
        coach_name: "Jane Doe".to_owned(),
        public_professional_email: "jdoe@testhigh.edu".to_owned(),
        ad_name: "John Smith".to_owned(),
        ad_email: "jsmith@testhigh.edu".to_owned(),
        source_urls: vec![
            "https://example.org/schools/test-high".to_owned(),
            "https://other.org/staff/test-high".to_owned(),
        ],
        last_observed: "2026-09-20".to_owned(),
    }
}

pub fn sample_claims(row: &RawContactRow) -> Vec<ContactClaimEvidence> {
    let coach = format!(
        "{} {} {} {} {} {}",
        row.school, row.state, row.sport, row.role, row.coach_name, row.public_professional_email
    );
    let ad = format!(
        "{} {} Athletic Director {} {}",
        row.school, row.state, row.ad_name, row.ad_email
    );
    [
        (
            ContactProofField::CoachName,
            &row.coach_name,
            &row.coach_name,
            row.role.as_str(),
            row.sport.as_str(),
            0,
            &coach,
        ),
        (
            ContactProofField::PublicProfessionalEmail,
            &row.public_professional_email,
            &row.coach_name,
            row.role.as_str(),
            row.sport.as_str(),
            0,
            &coach,
        ),
        (
            ContactProofField::AdName,
            &row.ad_name,
            &row.ad_name,
            "Athletic Director",
            "",
            1,
            &ad,
        ),
        (
            ContactProofField::AdEmail,
            &row.ad_email,
            &row.ad_name,
            "Athletic Director",
            "",
            1,
            &ad,
        ),
    ]
    .into_iter()
    .map(
        |(field, value, person, role, sport, source, span)| ContactClaimEvidence {
            field,
            value: value.clone(),
            person: person.clone(),
            role: role.to_owned(),
            sport: sport.to_owned(),
            school: row.school.clone(),
            state: row.state.clone(),
            source_url: row.source_urls[source].clone(),
            claimed_observed_on: row.last_observed.clone(),
            source_sha256: format!("{:x}", Sha256::digest(span.as_bytes())),
            fetched_at: "2026-09-21T00:00:00Z".to_owned(),
            span: span.clone(),
        },
    )
    .collect()
}

pub fn write_raw_csv(dir: &TempDir, row: &RawContactRow) -> TestResult<PathBuf> {
    let path = dir.path().join("contacts.csv");
    let mut writer = csv::Writer::from_path(&path)?;
    writer.write_record(CONTACT_COLUMNS)?;
    writer.write_record([
        &row.school,
        &row.city,
        &row.state,
        &row.sport,
        &row.role,
        &row.coach_name,
        &row.public_professional_email,
        &row.ad_name,
        &row.ad_email,
        &row.source_urls.join(" "),
        &row.last_observed,
    ])?;
    writer.flush()?;
    Ok(path)
}

pub fn write_staged(
    dir: &TempDir,
    row: &RawContactRow,
    claims: &[ContactClaimEvidence],
) -> TestResult<PathBuf> {
    Ok(stage_verified_contacts(&dir.path().join("stage"), [(row, claims)])?.staging_dir)
}

pub fn reseal(directory: &Path, rows: usize) -> TestResult {
    let manifest = Manifest {
        format_version: 1,
        verified_rows: rows,
        csv_sha256: format!(
            "{:x}",
            Sha256::digest(std::fs::read(directory.join("contacts.csv"))?)
        ),
        evidence_sha256: format!(
            "{:x}",
            Sha256::digest(std::fs::read(
                directory.join("contacts.csv.evidence.jsonl")
            )?)
        ),
    };
    std::fs::write(
        directory.join("manifest.json"),
        serde_json::to_vec(&manifest)?,
    )?;
    Ok(())
}
