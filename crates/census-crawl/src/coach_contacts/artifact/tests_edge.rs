use super::tests::helpers::{sample_claims, sample_row, TestResult};
use super::{stage_verified_contacts, ContactArtifactError};
use census_domain::model::RawContactRow;

#[path = "tests/edge_cases.rs"]
mod edge_cases;

#[test]
fn email_school_and_role_changes_cannot_reuse_prior_claims() -> TestResult {
    let mutations: [fn(&mut RawContactRow); 3] = [
        |row| row.public_professional_email = "JDOE@TESTHIGH.EDU".to_owned(),
        |row| row.school = "Changed High".to_owned(),
        |row| row.role = "Assistant Coach".to_owned(),
    ];
    for mutate in mutations {
        let dir = tempfile::tempdir()?;
        let mut row = sample_row();
        let original_claims = sample_claims(&row);
        mutate(&mut row);
        check!(matches!(
            stage_verified_contacts(
                &dir.path().join("stage"),
                [(&row, original_claims.as_slice())]
            ),
            Err(ContactArtifactError::Domain(_))
        ));
    }
    Ok(())
}
