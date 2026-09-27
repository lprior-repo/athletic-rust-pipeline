use super::{read_raw_contacts, read_verified_contacts, stage_verified_contacts, ContactArtifactError, CONTACT_COLUMNS};
use census_domain::model::{ContactClaimEvidence, RawContactRow};
pub(super) mod helpers;
use helpers::{sample_claims, sample_row, write_raw_csv, write_staged, reseal};
mod limits;

#[test]
fn raw_contacts_preserve_fields_and_all_source_urls() {
    let dir = tempfile::tempdir().unwrap();
    let row = sample_row();
    assert_eq!(read_raw_contacts(&write_raw_csv(&dir, &row)).unwrap(), vec![row]);
}

#[test]
fn missing_short_and_incorrect_headers_are_rejected() {
    for (csv, actual) in [
        ("", 0), ("school,city,state\n", 3),
        ("School,City,State,Sport,Role,Coach_Name,Email,AD_Name,AD_Email,Source_Urls,Last_Observed\n", 11),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("contacts.csv");
        std::fs::write(&path, csv).unwrap();
        assert!(matches!(read_raw_contacts(&path), Err(ContactArtifactError::HeaderMismatch { expected: 11, actual: found, .. }) if found == actual));
    }
}

#[test]
fn header_only_raw_csv_and_empty_staged_artifact_have_zero_rows() {
    let dir = tempfile::tempdir().unwrap();
    let raw = dir.path().join("raw.csv");
    std::fs::write(&raw, CONTACT_COLUMNS.join(",") + "\n").unwrap();
    assert_eq!(read_raw_contacts(&raw).unwrap(), Vec::<RawContactRow>::new());
    let artifact = stage_verified_contacts(&dir.path().join("stage"), std::iter::empty::<(&RawContactRow, &[ContactClaimEvidence])>()).unwrap();
    assert_eq!(read_verified_contacts(&artifact.staging_dir).unwrap().row_count(), 0);
}

#[test]
fn existing_empty_staging_directory_is_not_reused() {
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(stage_verified_contacts(dir.path(), std::iter::empty()), Err(ContactArtifactError::StagingPathExists { .. })));
}

#[test]
fn refusing_replacement_preserves_the_prior_artifact() {
    let dir = tempfile::tempdir().unwrap();
    let row = sample_row();
    let path = write_staged(&dir, &row, &sample_claims(&row));
    let mut replacement = row.clone();
    replacement.coach_name = "Different Person".to_owned();
    let claims = sample_claims(&replacement);
    assert!(matches!(stage_verified_contacts(&path, [(&replacement, claims.as_slice())]), Err(ContactArtifactError::StagingPathExists { .. })));
    assert_eq!(read_verified_contacts(&path).unwrap().rows()[0].row(), &row);
}

#[test]
fn quoted_unicode_and_url_fields_roundtrip_without_normalization() {
    let mutations: [fn(&mut RawContactRow); 5] = [
        |row| row.coach_name = "Doe, Jané".to_owned(),
        |row| row.ad_name = "Smith \"The Chief\" Jr".to_owned(),
        |row| row.school = "High\nSchool".to_owned(),
        |row| row.source_urls[0] = "https://example.org/schools/test-high,extra".to_owned(),
        |row| row.source_urls = vec!["https://first.org/school/1/".to_owned(), "https://second.org/staff/2".to_owned(), "https://third.org/directory/3".to_owned()],
    ];
    for mutate in mutations {
        let dir = tempfile::tempdir().unwrap();
        let mut row = sample_row();
        mutate(&mut row);
        let claims = sample_claims(&row);
        let path = write_staged(&dir, &row, &claims);
        let artifact = read_verified_contacts(&path).unwrap();
        assert_eq!(artifact.row_count(), 1);
        assert_eq!(artifact.rows()[0].row(), &row);
        assert_eq!(artifact.rows()[0].claims(), claims);
    }
}

#[test]
fn raw_eleven_column_csv_cannot_be_read_as_verified() {
    let dir = tempfile::tempdir().unwrap();
    write_raw_csv(&dir, &sample_row());
    std::fs::write(dir.path().join("contacts.csv.evidence.jsonl"), "").unwrap();
    reseal(dir.path(), 1);
    assert!(matches!(read_verified_contacts(dir.path()), Err(ContactArtifactError::HeaderMismatch { expected: 12, actual: 11, .. })));
}
