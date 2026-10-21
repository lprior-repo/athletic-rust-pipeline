use census_domain::public_identity_qualification::PublicIdentityQualificationVersion;

use crate::PublicIdentityError;

use super::*;

#[test]
fn public_identity_historical_v1_fixture_is_refused_without_a_lineage() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("historical");
    let result = crate::public_identity::create_historical_test_fixture(
        &root,
        PublicIdentityQualificationVersion::SourceBoundV1,
    );
    check!(matches!(
        result,
        Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "removed public identity V1 declaration"
        )))
    ));
    Ok(())
}
