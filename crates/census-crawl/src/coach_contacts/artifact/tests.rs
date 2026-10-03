use super::{
    read_raw_contacts, read_verified_contacts, stage_verified_contacts, ContactArtifactError,
    CONTACT_COLUMNS,
};
use census_domain::model::{ContactClaimEvidence, RawContactRow};
pub(super) mod helpers;
use helpers::{reseal, sample_claims, sample_row, write_raw_csv, write_staged, TestResult};
mod limits;

#[test]
fn raw_contacts_preserve_fields_and_all_source_urls() -> TestResult {
    let dir = tempfile::tempdir()?;
    let row = sample_row();
    check!(eq;
        read_raw_contacts(&write_raw_csv(&dir, &row)?)?,
        vec![row]
    );
    Ok(())
}

#[test]
fn missing_short_and_incorrect_headers_are_rejected() -> TestResult {
    for (csv, actual) in [
        ("", 0), ("school,city,state\n", 3),
        ("School,City,State,Sport,Role,Coach_Name,Email,AD_Name,AD_Email,Source_Urls,Last_Observed\n", 11),
    ] {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("contacts.csv");
        std::fs::write(&path, csv)?;
        check!(matches!(read_raw_contacts(&path), Err(ContactArtifactError::HeaderMismatch { expected: 11, actual: found, .. }) if found == actual));
    }
    Ok(())
}

#[test]
fn header_only_raw_csv_and_empty_staged_artifact_have_zero_rows() -> TestResult {
    let dir = tempfile::tempdir()?;
    let raw = dir.path().join("raw.csv");
    std::fs::write(&raw, CONTACT_COLUMNS.join(",") + "\n")?;
    check!(eq;
        read_raw_contacts(&raw)?,
        Vec::<RawContactRow>::new()
    );
    let artifact = stage_verified_contacts(
        &dir.path().join("stage"),
        std::iter::empty::<(&RawContactRow, &[ContactClaimEvidence])>(),
    )?;
    check!(eq;
        read_verified_contacts(&artifact.staging_dir)
            ?
            .row_count(),
        0
    );
    Ok(())
}

#[test]
fn existing_empty_staging_directory_is_not_reused() -> TestResult {
    let dir = tempfile::tempdir()?;
    check!(matches!(
        stage_verified_contacts(dir.path(), std::iter::empty()),
        Err(ContactArtifactError::StagingPathExists { .. })
    ));
    Ok(())
}

#[test]
fn refusing_replacement_preserves_the_prior_artifact() -> TestResult {
    let dir = tempfile::tempdir()?;
    let row = sample_row();
    let path = write_staged(&dir, &row, &sample_claims(&row))?;
    let mut replacement = row.clone();
    replacement.coach_name = "Different Person".to_owned();
    let claims = sample_claims(&replacement);
    check!(matches!(
        stage_verified_contacts(&path, [(&replacement, claims.as_slice())]),
        Err(ContactArtifactError::StagingPathExists { .. })
    ));
    check!(eq; read_verified_contacts(&path)?.rows()[0].row(), &row);
    Ok(())
}

#[test]
fn quoted_unicode_and_url_fields_roundtrip_without_normalization() -> TestResult {
    let mutations: [fn(&mut RawContactRow); 5] = [
        |row| row.coach_name = "Doe, Jané".to_owned(),
        |row| row.ad_name = "Smith \"The Chief\" Jr".to_owned(),
        |row| row.school = "High\nSchool".to_owned(),
        |row| row.source_urls[0] = "https://example.org/schools/test-high,extra".to_owned(),
        |row| {
            row.source_urls = vec![
                "https://first.org/school/1/".to_owned(),
                "https://second.org/staff/2".to_owned(),
                "https://third.org/directory/3".to_owned(),
            ]
        },
    ];
    for mutate in mutations {
        let dir = tempfile::tempdir()?;
        let mut row = sample_row();
        mutate(&mut row);
        let claims = sample_claims(&row);
        let path = write_staged(&dir, &row, &claims)?;
        let artifact = read_verified_contacts(&path)?;
        check!(eq; artifact.row_count(), 1);
        check!(eq; artifact.rows()[0].row(), &row);
        check!(eq; artifact.rows()[0].claims(), claims);
    }
    Ok(())
}

#[test]
fn raw_eleven_column_csv_cannot_be_read_as_verified() -> TestResult {
    let dir = tempfile::tempdir()?;
    write_raw_csv(&dir, &sample_row())?;
    std::fs::write(dir.path().join("contacts.csv.evidence.jsonl"), "")?;
    reseal(dir.path(), 1)?;
    check!(matches!(
        read_verified_contacts(dir.path()),
        Err(ContactArtifactError::HeaderMismatch {
            expected: 12,
            actual: 11,
            ..
        })
    ));
    Ok(())
}
